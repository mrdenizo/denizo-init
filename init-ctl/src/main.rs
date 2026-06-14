use std::io::{Read, Write};

fn main() {
    let socket = std::os::unix::net::UnixStream::connect("/run/denizo-init/service.sock");
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("incorrect number of args {} expected 1 or 2", args.len());
        return;
    }
    let mut cmd: String = String::new();
    match socket {
        Ok(mut ok) => {
            cmd += &args[1];
            if args.len() > 2 {
                cmd += " ";
                cmd += &args[2];
            }
            let result = ok.write_all(&cmd.into_bytes());
            if result.is_err() {
                println!("error writing to socket");
            }
            else if result.is_ok() {
                let mut buf = [0u8; 1024];
                let r = ok.read(&mut buf);
                if let Ok(s) = r {
                    let msg = String::from_utf8(buf[0..s].to_vec());
                    println!("{}", msg.unwrap());
                }
            }
        }
        Err(err) => {
            println!("could not connect to socket {}", err);
        }
    }
}