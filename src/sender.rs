use anyhow::anyhow;
use redis::AsyncCommands;
use serde_json::Value;

#[derive(Clone)]
pub struct Sender {
    redis_client: Option<redis::Client>,
    req_client: reqwest::Client,
}

impl Sender {
    pub fn init(redis_addr: &str) -> anyhow::Result<Self> {
        let s = Self::none_redis()?;

        let client = redis::Client::open(redis_addr)?;
        Ok(Self {
            redis_client: Some(client),
            ..s
        })
    }

    pub fn none_redis() -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        Ok(Self {
            redis_client: None,
            req_client: client,
        })
    }

    fn is_redis(&self) -> bool {
        self.redis_client.is_some()
    }

    fn get_redis_client(&self) -> redis::Client {
        self.redis_client.clone().unwrap()
    }

    async fn save_to_redis(&self, data: Value) -> anyhow::Result<()> {
        if !self.is_redis() {
            return Ok(());
        }
        let client = self.get_redis_client();
        // 电脑网络连接变化会导致获取连接一直卡住
        let mut conn = client.get_multiplexed_async_connection().await?;
        conn.lpush("solution", data.to_string()).await.map_err(|e| anyhow!("数据保存出错: {}", e))?;

        Ok(())
    }

    async fn send(&self, data: Value) -> anyhow::Result<()> {
        let url = "https://ec2-18-218-197-117.us-east-2.compute.amazonaws.com/validate";
        let res = self.req_client.post(url).json(&data)
            .header("authority", "ec2-18-217-135-255.us-east-2.compute.amazonaws.com")
            .header("accept-language", "zh-CN,zh;q=0.9,ko;q=0.8,ru;q=0.7")
            .header("cache-control", "no-cache")
            .header("origin", "https://bnb.reth.cc")
            .header("pragma", "no-cache")
            .header("referer", "https://bnb.reth.cc/")
            .header("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"macOS\"")
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "cross-site")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
        tracing::info!("准备提交 {}", data.get("solution").unwrap());
        match res.send().await {
            Err(e) => {
                tracing::error!("出错: {}", e);
                self.save_to_redis(data).await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Ok(res) => {
                let status = res.status();
                if status == 503 {
                    tracing::info!("状态码: {}，将重新放入redis或丢弃", status);
                    self.save_to_redis(data).await?;
                    return Ok(());
                }
                if status != 200 {
                    let text = res.text().await?;
                    tracing::info!("状态码: {status}, 返回值: {}", text);
                    return Ok(());
                }
                tracing::info!("状态码: {}", status);
            }
        }
        Ok(())
    }

    pub async fn put_to_send(&self, data: Value) -> anyhow::Result<()> {
        if self.is_redis() {
            self.save_to_redis(data).await?;
        } else {
            self.send(data).await?;
        }
        Ok(())
    }

    async fn async_run(self) -> anyhow::Result<()> {
        if !self.is_redis() {
            return Ok(());
        }
        let client = self.get_redis_client();
        let mut conn = client.get_multiplexed_async_connection().await?;
        loop {
            let data: String = match conn.rpop("solution", None).await {
                Ok(val) => val,
                Err(e) => {
                    tracing::error!("redis出错：{}，将重新获取连接", e);
                    conn = client.get_multiplexed_async_connection().await?;
                    continue;
                }
            };
            if data.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            let data: Value = serde_json::from_str(&data)?;
            self.send(data).await?;
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.clone().async_run().await {
                    tracing::error!("数据发送出错: {}，将在2秒后重启", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        });
        Ok(())
    }
}
