# 警告写在前面
1. 本软件代码中设定`每提交10次，会为作者的地址提交一次`，如果介意请自行修改源码进行编译，请参阅修改指南
2. 本软件不保证稳定性，不保证收益，不保证不会被封号

## 修改指南
将 `src/main.rs` 中的 `get_address(None)` 修改为 `get_address(Some(address_list.clone()))` 即可

# 提示
当前似乎只有提交返回成功才能算成功，所以取消了超时设定。网络慢可能会导致cpu空闲时间长，可以尝试多开软件

# 使用指南
## 通过源码使用

### 参考文档安装rust
https://www.rust-lang.org/tools/install

### 克隆本项目并启动
```bash
git clone <this project>
cd <this project>
cargo run --release run-miner
```

## Windows使用
本软件基于`Windows11`构建，其他版本未测试

1. 下载软件: https://github.com/zemelLeong/rbnb20-miner/releases/download/v0.1.0/rbnb20-miner.exe
2. 在本软件相同目录下创建 `address_list.txt` 文件，每行一个地址
3. 在命令行中执行 `./rbnb20-miner.exe run-miner` 命令即可启动
4. 查看当前收益，执行 `./rbnb20-miner.exe check-balance <address>`

## Linux使用
本软件基于`Ubuntu 22.04.3 LTS`构建，其他版本未测试

1. 下载软件: https://github.com/zemelLeong/rbnb20-miner/releases/download/v0.1.0/rbnb20-miner
2. 在本软件相同目录下创建 `address_list.txt` 文件，每行一个地址
3. 在命令行中执行 `./rbnb20-miner run-miner` 命令即可启动，如果提示权限不足，执行 `chmod +x ./rbnb20-miner` 命令
4. 查看当前收益，执行 `./rbnb20-miner check-balance <address>`

## 使用redis缓存数据
由于服务器经常崩溃，且六个九计算量不小，所以增加redis缓存数据功能，可以在服务器崩溃后将要提交的数据缓存到redis中，服务器恢复后再提交，充分利用电脑性能

1. 自行安装redis软件：https://redis.io/docs/install/install-redis/
2. 执行命令启动服务：`./rbnb20-miner[.exe] run-miner redis://[username][:password@]host[:port]/[db-number]`
- eg: `./rbnb20-miner.exe run-miner redis://:password@localhost:6379/0` 
