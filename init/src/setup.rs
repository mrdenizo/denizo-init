use std::{collections::HashMap, io::{Read, Write}, ops::Add, os::unix::net::UnixListener, thread, time::Duration};

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
                    let mut full_status = String::new();
                    for service in &mut *services {
                        let status = format!("{} is running: {}\n", service.0, service.1.is_service_running());
                        full_status += &status;
                    }
                    socket.write_all(&full_status.into_bytes());
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