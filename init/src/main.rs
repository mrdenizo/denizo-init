mod setup;
mod svc;

use std::{io::Write, thread, time::Duration};

const ANY_PID: nix::unistd::Pid = nix::unistd::Pid::from_raw(-1);

extern "C" fn catch_term(signal: nix::libc::c_int) {
    let socket = std::os::unix::net::UnixStream::connect("/run/denizo-init/service.sock");
    if let Ok(mut ok) = socket {
        ok.write_all(b"shutdown");
    }
}

fn main() {
    let denizo_init_ascii = r#"
  _____             _                _       _ _    
 |  __ \           (_)              (_)     (_) |   
 | |  | | ___ _ __  _ _______        _ _ __  _| |_  
 | |  | |/ _ \ '_ \| |_  / _ \      | | '_ \| | __| 
 | |__| |  __/ | | | |/ / (_) |     | | | | | | |_  
 |_____/ \___|_| |_|_/___\___/      |_|_| |_|_|\__| 
"#;

    println!("{}", denizo_init_ascii);
    thread::sleep(Duration::from_millis(2000));

    setup::setup_hostname();

    println!("getting init scripts...");

    let mut services = process_scripts("/etc/denizo-init/boot-scripts/", true);

    println!("setting up signal handler for shutting down");

    unsafe {
        let handler = nix::sys::signal::SigHandler::Handler(catch_term);
        let action = nix::sys::signal::SigAction::new(handler, nix::sys::signal::SaFlags::empty(), nix::sys::signal::SigSet::empty());

        nix::sys::signal::sigaction(nix::sys::signal::Signal::SIGTERM, &action).unwrap();
    }

    setup::setup_zombie_reaper();

    if std::fs::create_dir_all("/run/denizo-init").is_ok() {
        for i in 1..4 {
            let recv = std::os::unix::net::UnixListener::bind("/run/denizo-init/service.sock");
            match recv {
                Ok(ok) => {
                    setup::setup_socket_listener(&ok, &mut services);
                    break;
                }
                Err(err) => println!("could not connect to unix socket {} (attempt {}/3)", err, i)
            }
            thread::sleep(Duration::from_millis(1000*i));
        }
    }
    println!("could not create directory at /run/denizo-init (is /run mounted correctly)");
    println!("without socket denizo-init will not be able to correctly reboot/restart and manage services");

    println!("entering loop to prevent kernel panic");
    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}

fn process_syscall(reboot: bool, services: &std::collections::HashMap<String, crate::svc::Service>) {
    println!("caught term signal, preparing for shutting down...");
    println!("waiting for services to stop");

    for service in services.values() {
        if service.is_service_running() && !service.check_if_oneshot() {
            service.stop();
        }
    }

    println!("sending term signal to all other processes");

    let pid = nix::unistd::Pid::from_raw(-1);
    let result = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM);

    if let Err(err) = result {
        println!("could not send term to all processes {} shutdown aborted", err);
        return;
    }

    loop {
        let status = nix::sys::wait::waitpid(ANY_PID, Option::from(nix::sys::wait::WaitPidFlag::WNOHANG));
            match status {
            Ok(ok) => {
                if let Some(pid) = ok.pid() {
                    println!("still waiting for pid {}", pid.as_raw());
                }
            }
            Err(err) => {
                if matches!(err, nix::errno::Errno::ECHILD) {
                    println!("got all children exited successfully.");
                    break;
                }
                println!("error getting process status {}", err);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    println!("getting shutdown scripts...");

    process_scripts("/etc/denizo-init/shutdown-scripts/", true);

    println!("sending reboot to kernel");

    if reboot {
        nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT).unwrap();
        return;
    }
    nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF).unwrap();
}

fn process_scripts(dir: &str, autostart: bool) -> std::collections::HashMap<String, svc::Service>  {
    let scripts = std::fs::read_dir(dir);
    let mut services: std::collections::HashMap<String, svc::Service>  = std::collections::HashMap::new();
    match scripts {
        Ok(ok) => {
            let e = ok
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>();
            if let Ok(mut entry) = e {
                entry.sort();
                for file in entry {
                    if let Some(service) = process_entry(file.clone(), autostart) {
                        services.insert(String::from(file.file_name().unwrap().to_str().unwrap()), service);
                    }
                }
            }
        }
        Err(err) => process_error("Cannot get init script directory", err)
    }
    return services;
}

fn process_error(err_msg: &str, err: std::io::Error) {
    println!("{} {}!", err_msg, err);
}

fn process_entry(e: std::path::PathBuf, autostart: bool) -> Option<svc::Service> {
    if !e.is_file() {
        println!("{} is not a file.", e.to_str().unwrap());
        return None;
    }
    let name = e.clone();
    let mut service = svc::Service::new(name);
    if !e.to_str().unwrap().ends_with("disabled") && autostart {
        service.start();
    }
    return Option::from(service);
}