## denizo-init -- a small and simple init for Linux written in Rust

### Reverting back to legacy version
Use legacy branch.

### Building
> [!WARNING]
> This project will compile only on Linux
```bash
git clone https://github.com/mrdenizo/denizo-init.git
cd ./denizo-init
cargo build --release
```

### Installing (assuming release build)
```bash
cp ./target/release/denizo-init /usr/bin/
cp ./target/release/d-initctl /usr/bin/
```

Then add to kernel cmdline:
```
init=/usr/bin/denizo-init
```

> [!CAUTION]
> Without any configuration system will not boot, do not reboot without a way to get back.
### Location of boot scripts

Path of boot scripts:
`/etc/denizo-init/boot-scripts/`

Path of shutdown scripts:
`/etc/denizo-init/shutdown-scripts`

> [!NOTE]
> Shutdown scripts should be only -oneshot type only, or they will not work.

### Note on scripts
> Scripts are written fully in Bash

| Script type | Filename ends with |
|--|--|
| Oneshot | -oneshot |
| Default |  | 
| Auto Restart | -auto-restart |
| Disabled | -disabled |

> Init waits for -oneshot scripts to fully finish. Default and Auto Restart are not guaranteed to finish loading before proceeding to next script.

### Controlling services
Use `d-initctl` to control services, service name should same as filename of that service script.
```bash
d-initctl start 03-service.sh
d-initctl restart 03-service.sh
d-initctl stop 03-service.sh
```
> [!NOTE]
> Use `d-initctl refresh` after making any changes to `/etc/denizo-init/boot-scripts/` directory.

Use `d-initctl status` to get list of all available services.

Use `d-initctl status 03-service.sh` to display information about `03-service.sh`.

> [!CAUTION]
> d-initctl will not work without correctly mounted /run directory

### Shutting down
Execute command as root user:
```bash
kill 1
```
Or run as root user:
```bash
d-initctl reboot
d-initctl shutdown
```