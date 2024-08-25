use std::fs;
use std::path::Path;
use serde::Deserialize;
use toml;
use lazy_static::lazy_static;

#[derive(Deserialize)]
pub struct Config {
    pub database: Database,
}

#[derive(Deserialize)]
pub struct Database {
    pub db_type: String,
    sqlite: Option<Sqlite>,
    mysql: Option<Mysql>,
}

#[derive(Deserialize)]
struct Sqlite {
    path: String,
    file: String,
    name: String,
}

#[derive( Deserialize)]
struct Mysql {
    user: String,
    password: String,
    host: String,
    port: String,
    name: String,
}
impl Database {
    /// return sqlite file path
    pub  fn sqlite_path(&self)->String{
        let db: &Sqlite = self.sqlite.as_ref().unwrap();
        format!(
            "{}/{}",
            db.path, db.file
        )
    }

    /// return sqlite database name
    pub fn sqlite_db_name(&self)->String {
        format!("{}",self.sqlite.as_ref().unwrap().name)
    }
    /// return mysql url
    pub fn mysql_url(&self)->String{
        let db: &Mysql = self.mysql.as_ref().unwrap();
        // "mysql://root:root@localhost:3306/ssl_data"
        format!(
            "mysql://{}:{}@{}:{}/{}",
            db.user, db.password, db.host, db.port, db.name
        )
    }
    /// return database name
    pub fn mysql_db_name(&self)->String {
        format!("{}",self.mysql.as_ref().unwrap().name)
    }
}
impl Default for Sqlite {
    fn default() -> Self {
        Self{ path: "./".to_string(), file: "ssl_data.db".to_string(), name: "ssl_data".to_string() }
    }
}

impl Default for Mysql {
    fn default() -> Self {
        Self{ user: "root".to_string(), password: "root".to_string(), host: "localhost".to_string(), port: "3306".to_string(), name: "ssl_data".to_string() }
    }
}

// 添加 Default 实现来提供默认配置
impl Default for Config {
    fn default() -> Self {
        Config {
            database: Database{
                db_type:"mysql".to_string(),
                sqlite:Some(Sqlite::default()),
                mysql: Some(Mysql::default()),
            },
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = load_config();
}

pub fn load_config() -> Config {
    let current_path = env!("CARGO_MANIFEST_DIR");
    // 读取配置文件
    let file = format!("{}/../configs/config.toml", current_path);

    let toml_file = Path::new(&file);
    let content = if toml_file.exists() {
        fs::read_to_string(toml_file).expect("Failed to read configuration file")
    } else {
        String::new() // 如果文件不存在，返回空字符串
    };

    if content.is_empty() {
        // 返回默认配置
        Config::default()
    } else {
        toml::from_str(&content).expect("Failed to parse configuration file")
    }
}

#[cfg(test)]
mod tests {
    use super::load_config;

    #[test]
    fn test() {
        load_config();
    }
}