#![no_std]
#![no_main]

use core::{
    cmp::min, ffi::c_void
};

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_ktime_get_ns},
    macros::{map,uprobe, uretprobe}, 
    maps::{LruHashMap, RingBuf}, 
    programs::ProbeContext,
};
use aya_log_ebpf::{info,warn};
use aya_ebpf_bindings::helpers::bpf_probe_read_user;
use ssl_observer_common::{
    ProbeSslData,
    MAX_BUF_SIZE,
    READ,WRITE,
};

const ERROR_CODE:u32 = 0;
const SUCESS_CODE:u32 = 1;

const MAX_ENTRIES :u32 = 1024 * 2 ;
const MAX_BYTE_SIZE :u32 = 1024 * 1024 * 512;

#[map]
static mut START_NS: LruHashMap<u32, u64> = LruHashMap::<u32, u64>::with_max_entries(MAX_ENTRIES, 0);
#[map]
static mut BUFS: LruHashMap<u64,*const core::ffi::c_void> = LruHashMap::<u64,*const core::ffi::c_void>::with_max_entries(MAX_ENTRIES, 0);
#[map]
static mut SSL_DATA:RingBuf = RingBuf::with_byte_size(MAX_BYTE_SIZE, 0);

static TARGET_PID: u32 = 0;
static TARGET_UID: u32 = 0;


#[inline(always)]
fn trace_allowed(uid:u32,pid:u32) -> bool {
     /* 如果设置了目标进程ID且与当前进程ID不匹配，则不允许跟踪 */
     if TARGET_PID != 0 && TARGET_PID != pid{
        return false ;
     } 
     /* 如果设置了目标用户ID且与当前用户ID不匹配，则不允许跟踪 */
     if TARGET_UID != 0 {
         if TARGET_UID != uid {
            return  false;
         }
     } 
    true
}
// unsafe fn handshake(ctx: ProbeContext,_rw:u8)->Result<u32,u32> {
//     // Retrieve the combined process ID and thread group ID
//     let pid_tgid: u64 = bpf_get_current_pid_tgid();
//     // Destructure pid and tgid for clarity
//     let (pid, tgid) = ((pid_tgid as u32), (pid_tgid >> 32) as u32);
//     // Get the current user ID
//     let uid: u32 = bpf_get_current_uid_gid() as u32;

//     if !trace_allowed(uid, pid) {
//         info!(&ctx,"PID = {},UID = {}\nDon't allowed to trace !!!",pid,uid);
//         return Ok(1);
//     }

//     let ts:u64 = bpf_ktime_get_ns();

//     START_NS.insert(&tgid, &ts, 0).map_err(|x| x as u32)?;

//     Ok(SUCESS_CODE)
// }

// unsafe fn handshake_ret(ctx: ProbeContext,rw:u8) ->Result<u32,u32>{
//         let pid_tgid:u64 = bpf_get_current_pid_tgid();
//         let tgid = (pid_tgid >> 32) as u32;
//         let pid = pid_tgid as u32;
//         let uid:u32 = bpf_get_current_uid_gid() as u32;
    
//         let ts:u64 = bpf_ktime_get_ns();
    
//         if !trace_allowed(uid, pid) {
//             info!(&ctx,"PID = {},UID = {}\nDon't allowed to trace !!!",pid,uid);
//             return Ok(1);
//         }
    
//         // 开始的时间戳
//         let tsp_op: Option<*const u64> = START_NS.get_ptr(&tgid);
//         let tsp;
//         match tsp_op {
//             Some(value) => {
//                 tsp = *value;
//             },
//             None => return Ok(2),
//         }
    
//         if tsp == 0{
//             return Ok(3);
//         }
    
//         let ret :usize = ctx.ret().unwrap();
//         if ret <= 0  {    // handshake failed
//             return Ok(4);
//            } 

//         if let Some(mut entry) = SSL_DATA.reserve::<ProbeSslData>(0){
//             let data: *mut ProbeSslData=entry.as_mut_ptr();

//             let comm: [u8; 16] = bpf_get_current_comm().unwrap();

//             (*data) = ProbeSslData{
//                 timestamp_ns:ts,
//                 delta_ns:ts - tsp,
//                 pid,
//                 tgid,
//                 uid,
//                 buf_filled : 0,
//                 rw,
//                 is_handshake:true,
//                 comm,
//                 buf:[0;MAX_BUF_SIZE],
//                 len:0,
//             };
//             entry.submit(0);
//         }else {
//             info!(&ctx,"Reserve SSL_DATA failed!!!");
//         };
//         START_NS.remove(&tgid).map_err(|x| x as u32)?;     
    
//         Ok(0)
// }

unsafe fn ssl_enter(ctx: ProbeContext,_rw:u8)-> Result<u32, u32>{
    let current_pid_tgid: u64 = bpf_get_current_pid_tgid();
    let (tgid, pid) = ((current_pid_tgid >> 32) as u32, current_pid_tgid as u32);
    let uid: u32 = bpf_get_current_uid_gid() as u32;
    let timestamp :u64 = bpf_ktime_get_ns();

    if !trace_allowed(uid, pid) {
        info!(&ctx,"PID = {},UID = {}\nDon't allowed to trace !!!",pid,uid);
        return Ok(ERROR_CODE);
    }

    // int SSL_write(SSL *ssl, const void *buf, int num);
    // int SSL_read(SSL *ssl, void *buf, int num);
    // 返回 buf 的地址，其中 buf 未加密 
    let buf_ptr :*const core::ffi::c_void= ctx.arg(1).ok_or(1u32)?;

    BUFS.insert(&current_pid_tgid, &buf_ptr, 0).map_err(|x| x as u32)?;
    START_NS.insert(&tgid, &timestamp , 0).map_err(|x| x as u32)?;

    Ok(SUCESS_CODE)
}

unsafe fn ssl_exit(ctx: ProbeContext,rw:u8)-> Result<u32, u32> {
    let current_pid_tgid: u64 = bpf_get_current_pid_tgid();
    let (tgid, pid) = ((current_pid_tgid >> 32) as u32, current_pid_tgid as u32);
    let uid: u32 = bpf_get_current_uid_gid() as u32;
    let timestamp :u64 = bpf_ktime_get_ns();

    if !trace_allowed(uid, pid) {
        info!(&ctx,"PID = {},UID = {}\nDon't allowed to trace !!!",pid,uid);
        return Ok(ERROR_CODE);
    }
    
    // 开始的时间戳
    let start_time = match START_NS.get_ptr(&tgid) {
        Some(ptr) => *ptr,
        None => return Ok(ERROR_CODE), // Start time not found.
    };   
    if start_time == 0{
        return Ok(ERROR_CODE);
    }

    // 返回值是实际写入的字节数
    let ret_value_len: i32 = ctx.ret().unwrap();
    if ret_value_len <= 0 {
        return Ok(ERROR_CODE);
    }
    
    let size: usize = ret_value_len as usize;
    if size > MAX_BUF_SIZE {
        warn!(
            &ctx,
            "Size '{}' is greater then max allowed buffer size '{}', data will be truncated",
            size,
            MAX_BUF_SIZE
        );
    }
    // 取出 buf 的地址
    let buf_ptr: *const c_void = match BUFS.get(&current_pid_tgid) {
        Some(ptr) => *ptr,
        None => {
            info!(&ctx, "Failed to retrieve buffer pointer from BUFS.");
            return Ok(ERROR_CODE); // Or an error code depending on logic.
        }
    };

    let count: usize = min(size, MAX_BUF_SIZE);
    let comm: [u8; 16] = bpf_get_current_comm().unwrap_or([0; 16]);
    
    // let ring_buf = if rw == READ {
    //     SSL_READ_DATA.borrow_mut()
    // }else{
    //     SSL_WRITE_DATA.borrow_mut()
    // };

    if let Some(mut entry) = SSL_DATA.reserve::<ProbeSslData>(0){
        let data: *mut ProbeSslData = entry.as_mut_ptr();
        // 根据地址复制buf
        let ret = bpf_probe_read_user((*data).buf.as_mut_ptr() as * mut c_void,count.try_into().unwrap(),buf_ptr);

        //  0 表示操作成功
        (*data).buf_filled = if ret == 0 { 1 } else { 0 };
        (*data).len = count;
        (*data).timestamp_ns = timestamp;
        (*data).delta_ns = timestamp - start_time;
        (*data).pid = pid;
        (*data).tgid =tgid;
        (*data).uid = uid;
        (*data).rw = rw;
        (*data).is_handshake = false;
        (*data).comm = comm;
        
        // entry.write(data);
        entry.submit(0);     
    }else {
        info!(&ctx,"Reserve SSL_DATA failed!!!");
    };    

    // START_NS.remove(&tgid).map_err(|x| x as u32)?;

    Ok(0)
}

// #[uprobe]
// pub fn ssl_do_handshake(ctx: ProbeContext) -> u32 {
//     match unsafe {try_ssl_do_handshake(ctx)} {
//         Ok(ret) => ret,
//         Err(ret) => ret,
//     }
// }

// unsafe fn try_ssl_do_handshake(ctx: ProbeContext) -> Result<u32, u32> {
//     info!(&ctx, "function ssl_do_handshake called by libssl");
//     handshake(ctx,HANDSHAKE)
// }

// #[uretprobe]
// fn ssl_do_handshake_ret(ctx: ProbeContext) -> u32 {
//     match unsafe { try_ssl_do_handshake_ret(ctx) } {
//         Ok(ret) => ret,
//         Err(ret) => ret,
//     }
// }

// unsafe fn try_ssl_do_handshake_ret(ctx: ProbeContext) -> Result<u32, u32> {
//     info!(&ctx, "function ssl_do_handshake_ret called by libssl");
//     handshake_ret(ctx,HANDSHAKE)
// }


#[uprobe]
fn ssl_write(ctx: ProbeContext) -> u32 {
    match unsafe { try_ssl_write(ctx) } {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

unsafe fn try_ssl_write(ctx: ProbeContext) -> Result<u32, u32> {
    // info!(&ctx, "function ssl_write called by libssl");
    ssl_enter(ctx,WRITE)
}

#[uretprobe]
fn ssl_write_ret(ctx: ProbeContext) -> u32 {
    match unsafe { try_ssl_write_ret(ctx) } {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

unsafe fn try_ssl_write_ret(ctx: ProbeContext) -> Result<u32, u32> {
    // info!(&ctx, "function ssl_write_ret called by libssl");
    ssl_exit(ctx, WRITE)
}

#[uprobe]
fn ssl_read(ctx: ProbeContext) -> u32 {
    match unsafe { try_ssl_read(ctx) } {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

unsafe fn try_ssl_read(ctx: ProbeContext) -> Result<u32, u32> {
    // info!(&ctx, "function ssl_read called by libssl");
    ssl_enter(ctx, READ)
}

#[uretprobe]
fn ssl_read_ret(ctx: ProbeContext) -> u32 {
    match unsafe { try_ssl_read_ret(ctx) } {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

unsafe fn try_ssl_read_ret(ctx: ProbeContext) -> Result<u32, u32> {
    // info!(&ctx, "function ssl_read_ret called by libssl");
    ssl_exit(ctx, READ)
}


#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
