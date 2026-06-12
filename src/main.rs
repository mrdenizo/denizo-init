use std::{process::Command, thread, time::Duration};

const ANY_PID: nix::unistd::Pid = nix::unistd::Pid::from_raw(-1);
static SHUTDOWN_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

extern "C" fn catch_term(signal: nix::libc::c_int) {
    SHUTDOWN_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
    process_syscall(false);
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

    println!("setting system hostname from /etc/hostname.");
    {
        let hostname = std::fs::read_to_string("/etc/hostname");
        match hostname {
            Ok(ok) => {
                let result = nix::unistd::sethostname(ok.replace("\n", ""));
                if result.is_err() {
                    println!("could not set hostname.");
                }
                else {
                    println!("got hostname {}.", ok.replace("\n", ""));
                }
            }
            Err(err) => {
                println!("could not get system hostname {}", err)
            }
        }
    }

    println!("getting init scripts...");

    process_scripts("/etc/denizo-init/boot-scripts/");

    println!("setting up signal handler for shutting down");

    unsafe {
        let handler = nix::sys::signal::SigHandler::Handler(catch_term);
        let action = nix::sys::signal::SigAction::new(handler, nix::sys::signal::SaFlags::empty(), nix::sys::signal::SigSet::empty());

        nix::sys::signal::sigaction(nix::sys::signal::Signal::SIGTERM, &action).unwrap();
    }
    
    loop {
        let status = nix::sys::wait::waitpid(ANY_PID, Option::from(nix::sys::wait::WaitPidFlag::WNOHANG));
        match status {
            Ok(ok) => {
                if let Some(pid) = ok.pid() {
                    if pid.as_raw() == 0 {
                        println!("no children to reap.");
                        break;
                    }
                    else {

                    }
                }
            }
            Err(err) => {
                if matches!(err, nix::errno::Errno::ECHILD) {
                    println!("no children to reap");
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(1000));
    }
    println!("enering loop to prevent kernel panic");
    loop {
        thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn process_syscall(reboot: bool) {
    println!("caught term signal, preparing for shutting down...");
    println!("waiting for 2 seconds");
    thread::sleep(std::time::Duration::from_millis(2000));

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

    process_scripts("/etc/denizo-init/shutdown-scripts/");

    println!("sending reboot to kernel");

    if let Err(err) = nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF) {
        println!("could not send reboot to kernel {}", err);
        return;
    }
}

fn process_scripts(dir: &str) {
    let scripts = std::fs::read_dir(dir);
    match scripts {
        Ok(ok) => {
            let e = ok
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>();
            if let Ok(mut entry) = e {
                entry.sort();
                for file in entry {
                    process_entry(file);
                }
            }
        }
        Err(err) => process_error("Cannot get init script directory", err)
    }
}

fn process_error(err_msg: &str, err: std::io::Error) {
    println!("{} {}!", err_msg, err);
}

fn process_entry(e: std::path::PathBuf) {
    if !e.is_file() {
        println!("{} is not a file.", e.to_str().unwrap());
        return;
    }
    let name = e.clone();
    let mut str_name= "?";
    if let Some(t) = name.file_name() {
        str_name = t.to_str().unwrap();
    }
    println!("Starting up {}.", str_name);
    if str_name.ends_with("auto-restart") {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(e);
        std::thread::spawn(move || start_watcher(&mut cmd));
        return;
    }
    let result = Command::new("/bin/bash")
        .arg(e)
        .spawn();
    match result {
        Ok(mut ok) => {
            ok.wait();
            println!("Started {}.", str_name);
        },
        Err(err) => {
            println!("Failed to start up {} due to {}", str_name, err);
        }
    }
}

fn start_watcher(cmd: &mut std::process::Command) {
    while !SHUTDOWN_STARTED.load(std::sync::atomic::Ordering::Relaxed) {
        let child = cmd.spawn();
        match child {
            Ok(mut ok) => {
                ok.wait();
            },
            Err(err) => {
                process_error("could not spanwn restarting script", err);
            }
        }
    }
}