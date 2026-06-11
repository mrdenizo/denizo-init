## denizo-init -- a small and simple init for Linux written in Rust



### Building

> [!WARNING]
> This project will compile only on Linux



```bash

> git clone https://github.com/mrdenizo/denizo-init.git

> cd ./denizo-init

> cargo build --release

```



### Installing (assuming release build)

```

cp ./target/release/denizo-init /usr/bin/

```

> then add to kernel cmdline

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



### Note on scripts



> Scripts are written fully in Bash



> [!WARNING]
> Init will wait for every script status, so it will hang if something is not daemonized.



> [!NOTE]
> If `-auto-restart` is added to the end of filename Init will restart script if it exits and will wait for process in another thread (will not hang when process is not daemonized).
> **Auto-restart feature will start new process every time Bash exits, so be careful with detaching daemons.**

