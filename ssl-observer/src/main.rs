use aya::{include_bytes_aligned, maps::RingBuf, programs::UProbe, Bpf,};
use aya_log::BpfLogger;
use clap::Parser;
use log::{debug, info, warn};
use sqlx::{MySql, Pool};
use std::{ops::Deref, str};
use tokio::{io::unix::AsyncFd, signal};

use ssl_observer_common::ProbeSslData;
mod decode;
mod mysql_db;
// mod sqlite_db;
mod ui;
mod utils;
mod config;

use decode::print_buf;
use mysql_db::{init_db, insert_data};
use ui::display_data_async;

#[derive(Debug, Parser)]
#[clap(name = "SSL-Observer", long_about = "SSL Traffic Monitoring and Analysis Tool")]
struct Opt {
    /// Observe target PID only
    #[clap(short, default_value_t = 0)]
    pid: i32,
    /// Observe target UID only
    #[clap(short, default_value_t = 0)]
    uid: i32,
    /// Observe target Command only
    #[clap(short,default_value_t = String::from("all"))]
    command: String,
    /// Observe the specified library with the path,like "openssl:/path/libssl.so.1.1"
    #[clap(short , default_value_t = String::from("libssl"))]
    lib: String,
}

fn attach_openssl(bpf: &mut Bpf, lib: &String) -> Result<(), anyhow::Error> {
    // let ssl_do_handshake_program: &mut UProbe =
    //     bpf.program_mut("ssl_do_handshake").unwrap().try_into()?;
    // ssl_do_handshake_program.load()?;
    // ssl_do_handshake_program.attach(Some("SSL_do_handshake"), 0, lib, None)?;

    // let ssl_do_handshake_ret_program: &mut UProbe = bpf
    //     .program_mut("ssl_do_handshake_ret")
    //     .unwrap()
    //     .try_into()?;
    // ssl_do_handshake_ret_program.load()?;
    // ssl_do_handshake_ret_program.attach(Some("SSL_do_handshake"), 0, lib, None)?;
    // SSL_write
    let ssl_write_program: &mut UProbe = bpf.program_mut("ssl_write").unwrap().try_into()?;
    ssl_write_program.load()?;
    ssl_write_program.attach(Some("SSL_write"), 0, lib, None)?;

    let ssl_write_ret_program: &mut UProbe =
        bpf.program_mut("ssl_write_ret").unwrap().try_into()?;
    ssl_write_ret_program.load()?;
    ssl_write_ret_program.attach(Some("SSL_write"), 0, lib, None)?;
    // SSL_read
    let ssl_read_program: &mut UProbe = bpf.program_mut("ssl_read").unwrap().try_into()?;
    ssl_read_program.load()?;
    ssl_read_program.attach(Some("SSL_read"), 0, lib, None)?;

    let ssl_read_ret_program: &mut UProbe = bpf.program_mut("ssl_read_ret").unwrap().try_into()?;
    ssl_read_ret_program.load()?;
    ssl_read_ret_program.attach(Some("SSL_read"), 0, lib, None)?;
    Ok(())
}

fn attach_nss(bpf: &mut Bpf, lib: &String) -> Result<(), anyhow::Error> {
    // PR_Write
    let nss_write_program: &mut UProbe = bpf.program_mut("ssl_write").unwrap().try_into()?;
    nss_write_program.load()?;
    nss_write_program.attach(Some("PR_Write"), 0, lib, None)?;

    let nss_write_ret_program: &mut UProbe =
        bpf.program_mut("ssl_write_ret").unwrap().try_into()?;
    nss_write_ret_program.load()?;
    nss_write_ret_program.attach(Some("PR_Write"), 0, lib, None)?;

    // PR_Send
    // let nss_send_program: &mut UProbe = bpf.program_mut("ssl_write").unwrap().try_into()?;
    // nss_send_program.load()?;
    // nss_send_program.attach(Some("PR_Send"), 0, lib, None)?;

    // let nss_send_ret_program: &mut UProbe =
    //     bpf.program_mut("ssl_write_ret").unwrap().try_into()?;
    // nss_send_ret_program.load()?;
    // nss_send_ret_program.attach(Some("PR_Send"), 0, lib, None)?;

    // PR_Read
    let nss_read_program: &mut UProbe = bpf.program_mut("ssl_read").unwrap().try_into()?;
    nss_read_program.load()?;
    nss_read_program.attach(Some("PR_Read"), 0, lib, None)?;

    let nss_read_ret_program: &mut UProbe = bpf.program_mut("ssl_read_ret").unwrap().try_into()?;
    nss_read_ret_program.load()?;
    nss_read_ret_program.attach(Some("PR_Read"), 0, lib, None)?;

    // PR_Recv
    // let nss_recv_program: &mut UProbe = bpf.program_mut("ssl_read").unwrap().try_into()?;
    // nss_recv_program.load()?;
    // nss_recv_program.attach(Some("PR_Recv"), 0, lib, None)?;

    // let nss_recv_ret_program: &mut UProbe = bpf.program_mut("ssl_read_ret").unwrap().try_into()?;
    // nss_recv_ret_program.load()?;
    // nss_recv_ret_program.attach(Some("PR_Recv"), 0, lib, None)?;

    Ok(())
}

fn prepare_programs(bpf: &mut Bpf, opt: &Opt) -> Result<(), anyhow::Error> {
    let lib = &opt.lib;
    if lib == "libssl" {
        // default
        attach_openssl(bpf, &lib)?;
    } else {
        // 尝试找到冒号 ':' 的位置
        match lib.find(':') {
            Some(colon_index) => {
                // 分离两部分
                let prefix = &lib[..colon_index];
                let path = &lib[colon_index + 1..];

                // 去掉前面的 "openssl:" 或 "nss:" 和冒号
                let library_name = prefix.trim_end_matches(':').to_string();
                let file_path = path.to_string();

                // 根据 library_name 调用相应的函数
                match library_name.as_str() {
                    "openssl" => attach_openssl(bpf, &file_path)?,
                    "nss" => attach_nss(bpf, &file_path)?,
                    _ => return Err(anyhow::anyhow!("Unsupported library type")),
                }
            }
            None => {
                // 如果没有找到冒号，说明格式不符合预期
                println!("The provided string does not contain a colon ':'.");
                return Err(anyhow::anyhow!("No colon found in the input string"));
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::parse();
    env_logger::init();
    // 内存限制提升
    bump_memlock_rlimit()?;
    // 加载eBPF程序
    let mut bpf = load_bpf_program()?;
    // 初始化eBPF日志
    if let Err(e) = BpfLogger::init(&mut bpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {}", e);
    }
    // Hook 事件
    prepare_programs(&mut bpf, &opt)?;
    // 异步数据库连接池初始化
    let pool = init_db().await?;
    let events: RingBuf<&mut aya::maps::MapData> =
        RingBuf::try_from(bpf.map_mut("SSL_DATA").unwrap())?;
    // 建立异步的RingBuf，自动实现了epoll
    let mut events_fd: AsyncFd<RingBuf<&mut aya::maps::MapData>> = AsyncFd::new(events).unwrap();
    println!("Waiting for Ctrl-C...");
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Exiting...");
                break;
            },
            // 读取用户缓冲区中的 ProbeSslData 数据
            _ = async {
                read_event(&pool, &mut events_fd,&opt).await.unwrap();
                // read_event_batch(&pool, &mut events_fd,&opt).await.unwrap();
            }=>{}
        };
    }
    display_data_async(&pool).await;
    Ok(())
}

// 内存限制提升函数
fn bump_memlock_rlimit() -> Result<(), anyhow::Error> {
    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {}", ret);
    }
    Ok(())
}

// 异步加载eBPF程序的函数
fn load_bpf_program() -> Result<Bpf, anyhow::Error> {
    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Bpf::load_file` instead.

    #[cfg(debug_assertions)]
    let bpf = Bpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/debug/ssl-observer"
    ))?;

    #[cfg(not(debug_assertions))]
    let bpf = Bpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/release/ssl-observer"
    ))?;

    Ok(bpf)
}

async fn read_event(
    pool: &Pool<MySql>,
    events_fd: &mut AsyncFd<RingBuf<&mut aya::maps::MapData>>,
    opt: &Opt,
) -> Result<(), anyhow::Error> {
    // 检测这个RingBuf是否异步可读
    let mut guard = events_fd.readable_mut().await?;
    let events: &mut RingBuf<&mut aya::maps::MapData> = guard.get_inner_mut();

    while let Some(ring_event) = events.next() {
        let data: ProbeSslData = unsafe {
            let item: &[u8] = ring_event.deref();
            let data_ptr: *const ProbeSslData = item.as_ptr() as *const ProbeSslData;
            // let data_ptr: *const ProbeSslData = ring_event.deref().as_ptr() as *const ProbeSslData;
            *data_ptr
        };

        insert_data(pool, &data).await.unwrap();
        print_buf(&data, &opt).await;
    }

    Ok(())
}
