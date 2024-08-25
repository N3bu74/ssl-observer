use chrono::{DateTime, Local};
use std::{
    io,
    time::{Duration, SystemTime},
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

// 将时间戳转换为日期时间字符串的函数
pub async fn convert_timestamp_to_date(timestamp_ns: u64) -> Result<String, sqlx::Error> {
    let timestamp: u64 = (timestamp_ns as f64 * 0.000000001) as u64;
    // let timestamp: u64 = timestamp_ns / 1_000_000_000;
    let datetime: DateTime<Local> = match calculate_specific_time(timestamp).await {
        Ok(dt) => dt.into(),
        Err(_) => {
            return Err(sqlx::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to calculate specific time",
            )))
        }
    };
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

// 清理命令行字符串的函数
pub fn sanitize_comm(comm: &[u8]) -> String {
    String::from_utf8_lossy(comm)
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

// 根据系统启动时间后的秒数偏移，计算具体时间点
pub async fn calculate_specific_time(offset_seconds: u64) -> Result<SystemTime, std::io::Error> {
    // 使用异步文件操作打开/proc/uptime文件
    let uptime_file = File::open("/proc/uptime").await?;
    let mut reader = BufReader::new(uptime_file);

    // 异步读取第一行
    // let uptime_line = time::timeout(time::Duration::from_secs(5), reader.lines().next_line()).await??.expect("Read line failed!!!");
    let mut uptime_line = String::new();

    // 异步读取第一行，Tokio的read_line方法本身就是异步的，不需要额外的timeout处理
    reader.read_line(&mut uptime_line).await?;

    // 解析系统运行秒数，这里我们只取整数部分
    let uptime_seconds: f64 = uptime_line
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No uptime data found"))?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid uptime data format"))?;

    // 当前时间减去系统运行秒数得到系统启动时间
    let boot_time = SystemTime::now() - Duration::from_secs_f64(uptime_seconds as f64);

    // 在系统启动时间基础上加上偏移秒数得到目标时间
    let target_time = boot_time + Duration::from_secs(offset_seconds);

    Ok(target_time)
}
