//! `pet-cli` 集成测试：用 `assert_cmd` 调用真实二进制，覆盖成功路径与退出码。

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn pet_cli_end_to_end() {
    // 1) chat：成功退出，输出包含 reply 字段与 echo 回复。
    Command::cargo_bin("pet-cli")
        .unwrap()
        .args(["chat", "你好"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"reply\""))
        .stdout(predicate::str::contains("echo: 你好"));

    // 2) memory list：成功退出，输出 entries 数组且包含预置条目。
    Command::cargo_bin("pet-cli")
        .unwrap()
        .args(["memory", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"entries\""))
        .stdout(predicate::str::contains("\"hello\""));

    // 3) state：成功退出，输出 mood / energy / last_interaction_at。
    Command::cargo_bin("pet-cli")
        .unwrap()
        .arg("state")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mood\""))
        .stdout(predicate::str::contains("\"neutral\""))
        .stdout(predicate::str::contains("\"energy\""))
        .stdout(predicate::str::contains("\"last_interaction_at\""));

    // 4) 未知子命令：用法错误，退出码 1。
    Command::cargo_bin("pet-cli")
        .unwrap()
        .arg("bogus")
        .assert()
        .code(1);
}
