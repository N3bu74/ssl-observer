#![no_std]

use core::mem::size_of;

pub const MAX_BUF_SIZE: usize = 1024 * 3
    - size_of::<(
        u64,
        u64,
        u32,
        u32,
        u32,
        u8,
        u8,
        bool,
        [u8; TASK_COMM_LEN],
        usize,
    )>();
// pub const MAX_BUF_SIZE: usize = 1024 * 16;
pub const TASK_COMM_LEN: usize = 16;

pub const READ: u8 = 0;
pub const WRITE: u8 = 1;
// pub const HANDSHAKE: u8 = 2;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct ProbeSslData {
    pub timestamp_ns: u64,         // 时间戳（纳秒）
    pub delta_ns: u64,             // 函数执行时间
    pub pid: u32,                  // 进程 ID
    pub tgid: u32,                 // 线程 ID
    pub uid: u32,                  // 用户 ID
    pub buf_filled: u8,            // 缓冲区是否填充
    pub rw: u8,                    // 读或写（0为读，1为写 ,2为 handshake ）
    pub is_handshake: bool,        // 是否是握手数据
    pub comm: [u8; TASK_COMM_LEN], // 进程名

    pub buf: [u8; MAX_BUF_SIZE], // 数据缓冲区
    pub len: usize,              // 读/写数据的长度
}

