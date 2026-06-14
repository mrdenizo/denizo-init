use std::{io::{Error, ErrorKind::Other}, path::PathBuf, process::Command, sync::{Arc, atomic::AtomicI32}, thread, time::Duration};

use nix::{sys::signal::Signal::SIGTERM, unistd::Pid};

pub struct Service {
    pid: Arc<std::sync::atomic::AtomicI32>,
    path: PathBuf,
    name: String,
    auto_restarting: bool,
    running: Arc<std::sync::atomic::AtomicBool>
}

impl Service {
    pub fn new(path: PathBuf) -> Self {
        let str_name = path.file_name().unwrap().to_str().unwrap();
        let auto_restarting = str_name.ends_with("auto-restart");
        Self { pid: Arc::new(AtomicI32::new(-2)), path: path.clone(), name: String::from(str_name), auto_restarting: auto_restarting, running: Arc::new(std::sync::atomic::AtomicBool::new(false)) }
    }
    pub fn start(&mut self) -> Result<(), Error> {
        if self.check_if_oneshot() {
            self.start_oneshot_service();
            return Ok(());
        }
        if self.pid.load(std::sync::atomic::Ordering::Relaxed) != -2 {
            return Err(Error::new(Other, format!("service {} is already running", self.name)));
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
            return Err(Error::new(Other, format!("cannot stop oneshot service {}", self.name)));
        }
        if self.pid.load(std::sync::atomic::Ordering::Relaxed) == -2 {
            return Err(Error::new(Other, format!("service {} is not running", self.name)));
        }
        self.stop()?;
        self.start()?;
        return Ok(());
    }
    pub fn stop(&self) -> Result<(), Error> {
        if self.check_if_oneshot() {
            return Err(Error::new(Other, format!("cannot stop oneshot service {}", self.name)));
        }
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        if self.pid.load(std::sync::atomic::Ordering::Relaxed) == -2 {
            return Err(Error::new(Other, format!("service {} is not running", self.name)));
        }
        let pid = Pid::from_raw(self.pid.load(std::sync::atomic::Ordering::Relaxed));
        if nix::sys::signal::kill(pid, SIGTERM).is_ok() {
            while self.pid.load(std::sync::atomic::Ordering::Relaxed) != -2 {
                thread::sleep(Duration::from_millis(500));
            }
        }
        return Ok(());
    }
    pub fn is_service_running(&self) -> bool {
        return self.pid.load(std::sync::atomic::Ordering::Relaxed) != -2;
    }
    pub fn check_if_oneshot(&self) -> bool {
        return self.name.ends_with("oneshot");
    }
    fn start_oneshot_service(&self) {
        println!("starting {}", self.name);
        let child = Command::new("/bin/bash").arg(&self.path).spawn();
        match child {
            Ok(mut ok) => {
                ok.wait();
                println!("started {}", self.name);
            },
            Err(err) => println!("error starting service {}", err)
        }
    }
    fn watcher(pid: Arc<std::sync::atomic::AtomicI32>, running: Arc<std::sync::atomic::AtomicBool>, path: PathBuf, name: String, auto_restart: bool) {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(path);
        running.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("staring {}", name);
        while running.load(std::sync::atomic::Ordering::Relaxed) {
            let result = cmd.spawn();
            match result {
                Ok(mut child) => {
                    pid.store(child.id().try_into().unwrap(), std::sync::atomic::Ordering::SeqCst);
                    child.wait();
                    if !auto_restart {
                        break;
                    }
                }
                Err(err) => {
                    println!("failed to start service {} due to {}", name, err);
                }
            }
        }
        pid.store(-2, std::sync::atomic::Ordering::SeqCst);
        println!("service {} stopped", name);
    }
}