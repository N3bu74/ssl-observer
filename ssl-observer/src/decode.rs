use async_compression::tokio::bufread::GzipDecoder;
use std::str;
use tokio::io::{AsyncReadExt, BufReader};

use crate::Opt;
use ssl_observer_common::ProbeSslData;

pub async fn print_buf(data: &ProbeSslData, _opt: &Opt) {
    if (&data).is_handshake == false {
        println!(
            "\nv----- DATA -----v\n{}\n>----- END DATA -----<",
            parse_http(&data.buf).await
        );

        // println!(
        //     "\nv----- DATA -----v\n{}\n>----- END DATA -----<",
        //     parse_utf8_or_hex(&data.buf[..data.len]).await.unwrap()
        // );

        // println!(
        //     "\nv----- DATA -----v\n{}\n>----- END DATA -----<",
        //     parse_hex(&data.buf)
        // );
    }
}

// async fn parse_utf8_or_hex(buf: &[u8]) -> String{
//     match str::from_utf8(buf) {
//         Ok(str_slice) => str_slice.to_string(),
//         Err(_) => {
//             buf.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("")
//         }
//     }
// }

// 输出十六进制
// fn parse_hex(buf: &[u8]) -> String {
//     buf.iter().map(|byte| format!("{:02x}", byte)).collect()
// }

async fn async_decode_gzip(content: &[u8]) -> Result<String, std::io::Error> {
    // 使用async_compression的GzipDecoder创建一个异步解压读取器
    let decoder = GzipDecoder::new(content);
    let mut decoder = BufReader::new(decoder);

    // 初始化一个字符串用于存储解压后的数据
    let mut decoded_content = String::new();

    // 异步读取解压后的内容到字符串
    decoder.read_to_string(&mut decoded_content).await?;

    Ok(decoded_content)
}

// gzip解码转换成字符串。
pub async fn parse_http(buf: &[u8]) -> String {
    // 1. 检查是否存在 "\r\n\r\n"，确定 header 和 content 的分界线
    let header_end = buf
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or(buf.len());
    let (header, mut content_start) = buf.split_at(header_end);

    // 2. 解析 header，获取 Content-Encoding 字段
    let content_encoding = str::from_utf8(header)
        .ok()
        .and_then(|header_str| {
            header_str
                .lines()
                .find(|line| line.to_lowercase().starts_with("content-encoding:"))
                .map(|line| line.trim().to_lowercase())
        })
        .unwrap_or_default();

    // 3. 如果 Content-Encoding 为 gzip，则对 content 部分进行解码
    if content_encoding.contains("gzip") {
        content_start = content_start.split_at(4).1; // Skip \r\n\r\n after header
        if let Ok(decoded_content) = async_decode_gzip(content_start).await {
            return format!(
                "{}{}{}",
                str::from_utf8(header).unwrap(),
                "\r\n\r\n",
                decoded_content
            );
        }
    }
    // 4. 组装并返回结果
    let header_str = String::from_utf8_lossy(header);
    let content_str = String::from_utf8_lossy(content_start);
    format!("{}{}", header_str, content_str)
}
