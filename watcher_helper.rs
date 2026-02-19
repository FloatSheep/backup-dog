use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

// 定义配置结构体
#[derive(Serialize, Deserialize)]
struct Config {
    src: Vec<String>,
    dst: Vec<String>,
}

// 文件哈希映射，用于跟踪文件是否被修改
type FileHashes = HashMap<PathBuf, u64>;

// 计算文件的简单哈希值
fn calculate_hash<P: AsRef<Path>>(path: P) -> Result<u64, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        std::hash::Hasher::write(&mut hasher, &buffer[..bytes_read]);
    }
    
    Ok(hasher.finish())
}

// 获取文件修改时间
fn get_modified_time<P: AsRef<Path>>(path: P) -> Result<std::time::SystemTime, std::io::Error> {
    let metadata = fs::metadata(path)?;
    metadata.modified()
}

// 比较源文件和目标文件的哈希值，判断是否需要备份
fn needs_backup<P: AsRef<Path>>(src_file: P, dst_file: P) -> bool {
    let src_path = src_file.as_ref();
    let dst_path = dst_file.as_ref();

    // 如果目标文件不存在，则需要备份
    if !dst_path.exists() {
        return true;
    }

    // 如果源文件不存在，则不需要处理（可能已删除）
    if !src_path.exists() {
        return false;
    }

    // 计算源文件和目标文件的哈希值
    match (calculate_hash(src_path), calculate_hash(dst_path)) {
        (Ok(src_hash), Ok(dst_hash)) => src_hash != dst_hash,
        _ => true, // 如果无法计算哈希值，默认认为需要备份
    }
}

// 增量备份函数 - 逐块比较文件并更新差异部分
fn incremental_backup<P: AsRef<Path>>(src_file: P, dst_file: P) -> Result<(), std::io::Error> {
    let src_path = src_file.as_ref();
    let dst_path = dst_file.as_ref();

    let mut src = fs::File::open(src_path)?;
    let mut dst = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(dst_path)?;

    let mut src_buffer = [0u8; 4096];
    let mut dst_buffer = [0u8; 4096];

    let mut position = 0;
    loop {
        let src_bytes_read = src.read(&mut src_buffer)?;
        let dst_bytes_read = dst.read(&mut dst_buffer)?;
        
        if src_bytes_read == 0 && dst_bytes_read == 0 {
            // 文件结束，如果目标文件更长，截断它
            if dst_bytes_read == 0 && src_bytes_read == 0 {
                dst.set_len(position as u64)?;
            }
            break;
        }

        // 确定要比较的字节数
        let min_bytes = std::cmp::min(src_bytes_read, dst_bytes_read);
        
        // 比较缓冲区内容
        if src_buffer[..min_bytes] != dst_buffer[..min_bytes] {
            // 发现差异，写入新内容
            dst.seek(SeekFrom::Start(position as u64))?;
            dst.write_all(&src_buffer[..min_bytes])?;
        }

        // 如果源文件更短，截断目标文件
        if src_bytes_read < dst_bytes_read {
            dst.set_len(position as u64 + src_bytes_read as u64)?;
            break;
        }

        position += min_bytes;

        // 如果已经读完源文件但还有目标文件内容，截断目标文件
        if src_bytes_read == 0 {
            dst.set_len(position as u64)?;
            break;
        }
    }

    Ok(())
}

// 执行单个文件的备份
fn backup_file<P: AsRef<Path>>(src_file: P, dst_file: P) -> Result<(), std::io::Error> {
    let src_path = src_file.as_ref();
    let dst_path = dst_file.as_ref();

    // 确保目标目录存在
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 使用增量备份
    incremental_backup(src_path, dst_path)
}

// 同步整个目录
fn sync_directories<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<(), Box<dyn std::error::Error>> {
    let src_path = src_dir.as_ref();
    let dst_path = dst_dir.as_ref();

    // 读取源目录中的所有文件
    for entry in fs::read_dir(src_path)? {
        let entry = entry?;
        let src_file = entry.path();
        
        if src_file.is_file() {
            let filename = src_file.file_name().unwrap();
            let dst_file = dst_path.join(filename);

            // 检查是否需要备份
            if needs_backup(&src_file, &dst_file) {
                println!("正在备份: {:?} -> {:?}", src_file, dst_file);
                backup_file(&src_file, &dst_file)?;
            }
        }
    }

    Ok(())
}

// 获取配置
fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}

// 设置文件监听器
fn setup_watcher(config: &Config) -> Result<(RecommendedWatcher, Receiver<Result<Event, notify::Error>>), Box<dyn std::error::Error>> {
    let (tx, rx) = channel();
    let watcher = RecommendedWatcher::new(tx, Config::default())?;

    // 监听所有源目录
    for src_path in &config.src {
        let src_path = Path::new(src_path);
        watcher.watch(src_path, RecursiveMode::Recursive)?;
        println!("开始监听目录: {:?}", src_path);
    }

    Ok((watcher, rx))
}

// 主监听循环
fn watch_loop(rx: Receiver<Result<Event, notify::Error>>, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("开始监听文件变化...");

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                match event.kind {
                    notify::EventKind::Create(_) | 
                    notify::EventKind::Modify(_) |
                    notify::EventKind::Remove(_) => {
                        
                        println!("检测到文件变化: {:?}", event.paths);
                        
                        // 对于每个配置的源目录，同步到对应的目标目录
                        for (src, dst) in config.src.iter().zip(config.dst.iter()) {
                            match sync_directories(src, dst) {
                                Ok(_) => println!("目录同步完成: {} -> {}", src, dst),
                                Err(e) => eprintln!("同步失败 {}: {} -> {}: {}", src, dst, e),
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => eprintln!("监听错误: {:?}", e),
            Err(e) => eprintln!("接收事件错误: {:?}", e),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config()?;
    
    if config.src.is_empty() || config.dst.is_empty() {
        eprintln!("错误: 配置文件中没有定义源目录或目标目录");
        std::process::exit(1);
    }

    if config.src.len() != config.dst.len() {
        eprintln!("错误: 源目录数量与目标目录数量不匹配");
        std::process::exit(1);
    }

    let (_, rx) = setup_watcher(&config)?;
    watch_loop(rx, config)?;

    Ok(())
}