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

    fn save_to_redis(&self, data: Value) -> anyhow::Result<()> {
        if !self.is_redis() {
            return Ok(());
        }
        let client = self.get_redis_client();
        let mut conn = client.get_connection()?;
        redis::cmd("LPUSH").arg("solution").arg(data.to_string()).query(&mut conn)?;

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
                self.save_to_redis(data)?;
            }
            Ok(res) => {
                let status = res.status();
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
            self.save_to_redis(data)?;
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
        let mut conn = client.get_connection()?;
        loop {
            let data: String = redis::cmd("RPOP").arg("solution").query(&mut conn)?;
            if data.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            let data: Value = serde_json::from_str(&data)?;
            self.send(data).await?;
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        tokio::spawn(self.async_run());
        Ok(())
    }
}
