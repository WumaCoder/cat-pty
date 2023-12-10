use nix::fcntl::{open, OFlag};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt};
use nix::sys::stat::Mode;
use nix::unistd::{close, dup, dup2, setsid, write};
use std::{ffi::CString, os::fd::AsRawFd, thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 打开伪终端主设备
    let master_fd = posix_openpt(OFlag::O_RDWR)?;

    // 将主设备授予子进程
    grantpt(&master_fd)?;

    // 解锁从设备
    unlockpt(&master_fd)?;

    // 获取从设备名称
    let slave_name = CString::new(unsafe { ptsname(&master_fd) }?.as_str())?;

    // 创建子进程
    match unsafe { nix::unistd::fork()? } {
        nix::unistd::ForkResult::Parent { child } => {
            // 关闭父进程中不需要的文件描述符
            println!("hello main");

            thread::spawn({
                let master_fd = master_fd.as_raw_fd();
                move || {
                    loop {
                        // 读取子进程的输出
                        let mut output_data = [0u8; 1024];
                        let len = nix::unistd::read(master_fd, &mut output_data).unwrap_or(0);
                        if len > 0 {
                            print!("{}", String::from_utf8_lossy(&output_data[..len]));
                        }
                    }
                }
            });

            write(master_fd.as_raw_fd(), "echo 'Hello, World!'".as_bytes()).unwrap();
            write(master_fd.as_raw_fd(), "\n".as_bytes()).unwrap();
            write(master_fd.as_raw_fd(), "zsh".as_bytes()).unwrap();
            write(master_fd.as_raw_fd(), "\n".as_bytes()).unwrap();
            write(master_fd.as_raw_fd(), "echo 'Hello, World!'".as_bytes()).unwrap();
            write(master_fd.as_raw_fd(), "\n".as_bytes()).unwrap();

            // 等待子进程完成
            let status = nix::sys::wait::waitpid(child, None)?;
            println!("子进程退出，状态码: {:?}", status);
        }
        nix::unistd::ForkResult::Child => {
            // 在子进程中创建新的会话并设置为控制终端
            setsid()?;

            // 打开从设备
            let slave_fd = open(slave_name.as_c_str(), OFlag::O_RDWR, Mode::empty())?;

            // 将从设备作为子进程的标准输入、输出、错误
            dup2(slave_fd, nix::libc::STDIN_FILENO)?;
            dup2(slave_fd, nix::libc::STDOUT_FILENO)?;
            dup2(slave_fd, nix::libc::STDERR_FILENO)?;

            println!("hello child");

            // 关闭不再需要的文件描述符
            close(slave_fd)?;
            close(master_fd.as_raw_fd())?;

            // 执行 Bash
            let cmd = CString::new("bash")?;
            let args = [cmd.as_c_str()];
            nix::unistd::execvp(cmd.as_c_str(), &args)?;

            // 如果 execvp 失败，退出子进程
            unsafe { nix::libc::_exit(1) };
        }
    }

    Ok(())
}
