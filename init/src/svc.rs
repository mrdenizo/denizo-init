use std::os::unix::process::CommandExt;
use std::sync::Arc;
use std::sync::atomic::{ AtomicI32, AtomicBool, Ordering };
use std::io::{ Error, ErrorKind };
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::process::Command;

use nix::unistd::{ self, Pid, ForkResult };
use nix::sys::{ wait, signal };
use nix::libc;


pub struct Service {
    pid: Arc<AtomicI32>,
    path: PathBuf,
    name: String,
    auto_restarting: bool,
    running: Arc<AtomicBool>
}

impl Service {
    pub fn new(path: PathBuf) -> Self {
        let str_name = path.file_name().unwrap().to_str().unwrap();
        let auto_restarting = str_name.ends_with("auto-restart");
        Self { 
            pid: Arc::new(AtomicI32::new(-2)),
            path: path.clone(),
            name: String::from(str_name),
            auto_restarting: auto_restarting,
            running: Arc::new(AtomicBool::new(false))
        }
    }
    pub fn start(&mut self) -> Result<(), Error> {
        if self.check_if_oneshot() {
            self.start_oneshot_service();
            return Ok(());
        }
        if self.pid.load(Ordering::Relaxed) != -2 {
            return Err(Error::new(ErrorKind::Other, format!("service {} is already running", self.name)));
        }
        let pid = self.pid.clone();
        let running = self.running.clone();
        let restarting = self.auto_restarting.clone();
        let path = self.path.clone();
        let name = self.name.clone();
        thread::spawn(move || Service::watcher(pid, running, path, name, restarting));
        return Ok(());
    }
    pub fn restart(&mut self) -> Result<(), Error> {
        if self.check_if_oneshot() {
            return Err(Error::new(ErrorKind::Other, format!("cannot stop oneshot service {}", self.name)));
        }
        if self.pid.load(Ordering::Relaxed) == -2 {
            return Err(Error::new(ErrorKind::Other, format!("service {} is not running", self.name)));
        }
        self.stop()?;
        self.start()?;
        return Ok(());
    }
    pub fn stop(&self) -> Result<(), Error> {
        if self.check_if_oneshot() {
            return Err(Error::new(ErrorKind::Other, format!("cannot stop oneshot service {}", self.name)));
        }
        self.running.store(false, Ordering::SeqCst);
        if self.pid.load(Ordering::Relaxed) == -2 {
            return Err(Error::new(ErrorKind::Other, format!("service {} is not running", self.name)));
        }
        let pid = Pid::from_raw(self.pid.load(Ordering::Relaxed));
        if signal::kill(pid, signal::SIGTERM).is_ok() {
            while self.pid.load(Ordering::Relaxed) != -2 {
                thread::sleep(Duration::from_millis(500));
            }
        }
        return Ok(());
    }
    pub fn is_service_running(&self) -> bool {
        return self.pid.load(Ordering::Relaxed) != -2;
    }
    pub fn check_if_oneshot(&self) -> bool {
        return self.name.ends_with("oneshot");
    }
    fn start_oneshot_service(&self) {
        println!("starting {}", self.name);
        Service::fork_and_exec(&self.name, &self.path, None);
    }
    fn watcher(pid: Arc<AtomicI32>, running: Arc<AtomicBool>, path: PathBuf, name: String, auto_restart: bool) {
        running.store(true, Ordering::SeqCst);
        println!("staring {}", name);
        while running.load(Ordering::Relaxed) {
            Service::fork_and_exec(&name, &path, Option::from(&pid));
            if !auto_restart {
                break;
            }
        }
        pid.store(-2, Ordering::SeqCst);
        println!("service {} stopped", name);
    }
    fn fork_and_exec(name: &String, path: &PathBuf, pid: Option<&Arc<AtomicI32>>) {
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Parent { child }) => {
                if let Some(id) = pid {
                    id.store(child.as_raw(), Ordering::SeqCst);
                }
                wait::waitpid(child, None);
            }
            Ok(ForkResult::Child) => {
                unistd::setsid().unwrap();
                unsafe { libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY); }
                let err = Command::new("/usr/bin/sh").arg(path).exec();
                println!("failed to start service {} due to {}", name, err);
                std::process::exit(1);
            }
            Err(err) => {
                println!("error forking {}", err);
            }
        }
    }
}