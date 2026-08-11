use std::{collections::HashMap, io::{Read, Write}, os::unix::net::UnixListener, thread, time::Duration};

pub fn setup_hostname() {
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
}
fn find_diff(services: &HashMap<String, crate::svc::Service>, new_services: &HashMap<String, crate::svc::Service>) -> (Vec<String>, Vec<String>) {
    let mut diff = std::vec::Vec::new();
    let mut unchanged = std::vec::Vec::new();
    for old_svc in services.keys() {
        if !new_services.contains_key(old_svc) {
            diff.push(old_svc.clone());
        }
        else {
            unchanged.push(old_svc.clone());
        }
    }
    return (diff, unchanged);
}
pub fn setup_socket_listener(recv: &UnixListener, services: &mut HashMap<String, crate::svc::Service>) {
    loop {
        match recv.accept() {
            Ok((mut socket, addr)) => {
                let mut buf = [0u8; 1024];
                let mut cmd = String::new();
                if let Ok(size) = socket.read(&mut buf) {
                    cmd = String::from_utf8(buf[0..size].to_vec()).unwrap();
                }
                else {
                    socket.write_all(b"error reading from socket");
                }
                if cmd == "reboot" {
                    socket.write_all(b"got reboot");
                    crate::process_syscall(true, &services);
                    return;
                }
                else if cmd == "shutdown" {
                    socket.write_all(b"got shutdown");
                    crate::process_syscall(false, &services);
                    return;
                }
                else if cmd == "status" {
                    let mut status = String::new();
                    for service in &mut *services {
                        status += &format!("{} is running: {}\n", service.0, service.1.is_service_running());
                    }
                    socket.write_all(&status.into_bytes());
                }
                else if cmd == "refresh" {
                    let mut new_services = crate::process_scripts("/etc/denizo-init/boot-scripts/", false);
                    let mut report = String::new();
                    let removed = find_diff(services, &new_services).0;
                    let added = find_diff(&new_services, services);
                    
                    for svc in removed {
                        if services[&svc].stop().is_ok() {
                            report += &format!("{} stopped\n", svc);
                        }
                        services.remove(&svc);
                        report += &format!("{} removed\n", svc);
                    }

                    for svc in added.0 {
                        services.insert(svc.clone(), new_services.remove(&svc).unwrap());
                        report += &format!("{} added\n", svc);
                    }
                    for svc in added.1 {
                        report += &format!("{} unchanged\n", svc);
                    }
                    socket.write_all(&report.into_bytes());
                }
                else if cmd.starts_with("start ") {
                    let name = &cmd[6..cmd.len()];
                    if let Some(svc) = services.get_mut(name) {
                        let result = svc.start();
                        if let Err(err) = result {
                            socket.write_all(&err.to_string().into_bytes());
                        }
                        else {
                            socket.write_all(&format!("started {}", name).into_bytes());
                        }
                    }
                    else {
                        socket.write_all(&format!("service {} does not exist", name).into_bytes());
                    }
                }
                else if cmd.starts_with("restart ") {
                    let name = &cmd[8..cmd.len()];
                    if let Some(svc) = services.get_mut(name) {
                        let result = svc.restart();
                        if let Err(err) = result {
                            socket.write_all(&err.to_string().into_bytes());
                        }
                        else {
                            socket.write_all(&format!("restarted {}", name).into_bytes());
                        }
                    }
                    else {
                        socket.write_all(&format!("service {} does not exist", name).into_bytes());
                    }
                }
                else if cmd.starts_with("stop ") {
                    let name = &cmd[5..cmd.len()];
                    if let Some(svc) = services.get_mut(name) {
                        let result = svc.stop();
                        if let Err(err) = result {
                            socket.write_all(&err.to_string().into_bytes());
                        }
                        else {
                            socket.write_all(&format!("stopped {}", name).into_bytes());
                        }
                    }
                    else {
                        socket.write_all(&format!("service {} does not exist", name).into_bytes());
                    }
                }
                else if cmd.starts_with("status ") {
                    let name = &cmd[7..cmd.len()];
                    if let Some(svc) = services.get(name) {
                        socket.write_all(&format!("{} is running: {}", name, svc.is_service_running()).into_bytes());
                    }
                    else {
                        socket.write_all(&format!("service {} does not exist", name).into_bytes());
                    }
                }
                else {
                    socket.write_all(&format!("unknown command {}", cmd).into_bytes());
                }
            }
            Err(err) => {
                println!("error handling command from socket {}", err);
            }
        }
    }
}
pub fn setup_zombie_reaper() {
    thread::spawn(|| reap_zombies());
}
fn reap_zombies() {
    loop {
        let status = nix::sys::wait::waitpid(crate::ANY_PID, Option::from(nix::sys::wait::WaitPidFlag::WNOHANG));
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
}