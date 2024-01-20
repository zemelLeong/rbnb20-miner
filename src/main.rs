use std::time::Duration;
use anyhow::{anyhow, Result};
use ethers::abi::AbiEncode;
use ethers::utils::keccak256;
use rayon::iter::ParallelIterator;
use clap::{Args, Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// 读取 `address_list.txt` 文件，初始化地址列表
fn init_address_list() -> Vec<String> {
    let content = std::fs::read_to_string("address_list.txt")
        .map_err(|e| anyhow!("读取地址文件 address_list.txt 出错 {}", e))
        .unwrap();
    let address_list = content
        .lines()
        .map(|line| line.trim().to_lowercase())
        .filter(|line| {
            !line.is_empty()
        })
        .collect::<Vec<_>>();
    address_list
}

/// 随机选择一个地址返回
fn get_address(addr_list: Option<Vec<String>>) -> String {
    match addr_list {
        None => {
            tracing::warn!("将为作者提交一次");
            "0x15FCEA85bEdA82e9e186d968C1CDC2c96865f917".to_lowercase()
        }
        Some(list) => {
            let index = rand::random::<usize>() % list.len();
            list[index].to_lowercase()
        }
    }
}

const DIFFICULTY: &str = "0x999999";

fn get_hash(addr: &str) -> Option<String> {
    let random_value = rand::random::<[u8; 32]>();
    let potential_solution = random_value.encode_hex();

    let fixed_part = {
        let tmp = "72424e4200000000000000000000000000000000000000000000000000000000000000000000000000000000";
        hex::decode(tmp).unwrap()
    };
    let address = {
        let tmp = addr.trim_start_matches("0x");
        hex::decode(tmp).unwrap()
    };
    let data = [random_value.to_vec(), fixed_part, address].concat();

    let hashed_solution = keccak256(data).encode_hex();

    if hashed_solution.starts_with(DIFFICULTY) {
        Some(potential_solution)
    } else {
        None
    }
}

async fn find_solution(address: &str) -> Result<String> {
    let res = rayon::iter::repeat(address)
        .map(get_hash)
        .find_any(|hash| hash.is_some())
        .flatten()
        .ok_or_else(|| anyhow::anyhow!("出错"))?;

    Ok(res)
}

#[test]
fn hash_verify() {
    let list = init_address_list();
    let address = get_address(Some(list));
    let ps_hex =
        hex::decode("de2c754b3ef38f4dbf478f5d0ee644e36952dc4628a48350d856cc7745ca61c9").unwrap();

    let fixed_part =
        "72424e4200000000000000000000000000000000000000000000000000000000000000000000000000000000";
    let fixed_part_hex = hex::decode(fixed_part).unwrap();
    let addr_hex = hex::decode(address.trim_start_matches("0x")).unwrap();

    let data = [ps_hex, fixed_part_hex, addr_hex].concat();

    let hashed_solution = keccak256(data).encode_hex();
    tracing::info!("{}", hashed_solution);
}

#[tokio::test]
async fn test_find_solution() {
    let list = init_address_list();
    let address = get_address(Some(list));
    let res = find_solution(&address).await;
    tracing::info!("{:?}", res);
}

async fn run_miner() -> Result<()> {
    let agent = reqwest::Client::builder()
        // .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()?;
    let address_list = init_address_list();
    let mut counter = 0;
    loop {
        counter += 1;
        let address = {
            // 完成10次，帮助一次作者
            if counter % 10 == 0 {
                get_address(None)
            } else {
                get_address(Some(address_list.clone()))
            }
        };
        let solution = find_solution(&address).await?;
        // tracing::info!("solution值: {}", solution);
        let res = agent.post("https://ec2-18-218-197-117.us-east-2.compute.amazonaws.com/validate")
            .json(&serde_json::json!({
                "solution": solution,
                "challenge": "0x72424e4200000000000000000000000000000000000000000000000000000000",
                "address": address,
                "difficulty": DIFFICULTY,
                "tick": "rBNB",
            }))
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
        tracing::info!("准备提交 {solution}");
        match res.send().await {
            Err(e) => {
                tracing::info!("出错: {}", e);
            }
            Ok(res) => {
                let status = res.status();
                if counter % 10 == 0 || status != 200 {
                    let text = res.text().await?;
                    tracing::info!("状态码: {status}, 返回值: {}", text);
                    continue;
                }
                tracing::info!("状态码: {}", status);
            }
        }
    }
}

async fn get_balance(address: &str) {
    let agent = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let url = format!("https://ec2-18-218-197-117.us-east-2.compute.amazonaws.com/balance?address={address}");
    let res = agent.get(url);
    match res.send().await {
        Ok(res) => {
            let status = res.status();
            if status == 200 {
                let text = res.text().await.unwrap();
                tracing::info!("{}", text);
            }
        }
        Err(err) => {
            tracing::error!("出错: {}", err);
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    RunMiner,
    CheckBalance(AddrArg),
}

#[derive(Args, Debug)]
struct AddrArg {
    address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(tracing_subscriber::fmt::layer())
        .init();
    let args = Cli::parse();
    match args.command {
        Commands::RunMiner => {
            tracing::info!("开始运行");
            run_miner().await?;
        }
        Commands::CheckBalance(AddrArg{ address }) => {
            loop {
                get_balance(&address).await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
