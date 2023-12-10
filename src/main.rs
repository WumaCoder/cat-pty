use nix::{
    libc,
    sys::wait::{waitpid, WaitStatus},
    unistd::{close, dup2, execvp, fork, pipe, read, ForkResult},
};
use std::{ffi::CString, io::Write, os::unix::io::AsRawFd};

const BUFFER_SIZE: usize = 4096;

fn main() {
    let (input_pipe_read, input_pipe_write) = pipe().expect("创建输入管道失败");
    let (output_pipe_read, output_pipe_write) = pipe().expect("创建输出管道失败");

    println!("hello main");
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Parent process
            println!("main process");

            // 关闭不必要的管道端口
            close(input_pipe_write).expect("关闭输入管道写端失败");
            close(output_pipe_write).expect("关闭输出管道写端失败");

            // 读取输出
            let mut buffer = [0u8; BUFFER_SIZE];
            loop {
                match read(output_pipe_read, &mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        print!(">>{}", output);
                    }
                    Err(e) => {
                        eprintln!("从输出管道读取数据时发生错误 {:?}", e);
                        break;
                    }
                }
            }

            // 等待子进程完成
            match waitpid(child, None) {
                Ok(WaitStatus::Exited(_, status)) => {
                    println!("子进程退出，状态码: {}", status);
                }
                _ => {
                    eprintln!("等待子进程退出时发生错误");
                }
            }
        }
        Ok(ForkResult::Child) => {
            // Child process
            println!("child process");

            // 关闭不必要的文件描述符
            close(input_pipe_write).expect("关闭输入管道写端失败");
            close(output_pipe_read).expect("关闭输出管道读端失败");

            // 重定向标准输入和输出
            if let Err(err) = dup2(input_pipe_read, libc::STDIN_FILENO) {
                eprintln!("重定向标准输入失败: {}", err);
                unsafe { libc::exit(1) };
            }
            if let Err(err) = dup2(output_pipe_write, libc::STDOUT_FILENO) {
                eprintln!("重定向标准输出失败: {}", err);
                unsafe { libc::exit(1) };
            }

            // 关闭子进程不需要的管道端口
            close(input_pipe_read).expect("关闭输入管道读端失败");
            close(output_pipe_write).expect("关闭输出管道写端失败");

            // 执行 Bash 命令
            let cmd = CString::new("ping").expect("Failed to create CString");
            let echo = CString::new("114.114.114.114").expect("Failed to create CString");
            let args = [cmd.as_c_str(), echo.as_c_str()];
            execvp(cmd.as_c_str(), &args).expect("Failed to execute command");
        }
        Err(_) => {
            eprintln!("Fork failed");
        }
    }
    println!("hello main");
}
