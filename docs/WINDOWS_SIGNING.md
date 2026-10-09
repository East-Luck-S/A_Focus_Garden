# Windows 公开发行签名

公开 Windows 安装包必须使用长期稳定的 Authenticode 发布者身份。签名不是运行应用的前提，但 Focus Garden 的公开发布 Gate 要求应用 EXE 与 NSIS 安装包都具有有效签名。

## 准备

1. 从可信证书颁发机构取得代码签名证书；普通 TLS/SSL 证书不可用。
2. 将带私钥的证书导入 `Cert:\CurrentUser\My`，并确认有效期与发布者名称。
3. 复制 `src-tauri/tauri.signing.example.conf.json` 为被 Git 忽略的 `src-tauri/tauri.signing.local.conf.json`。
4. 填入证书 40 位十六进制 thumbprint，以及证书颁发机构提供的时间戳 URL；不得把密码、PFX 或私钥写入配置。

只读列出当前用户可用代码签名证书：

```powershell
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
  Select-Object Subject, Thumbprint, NotAfter, HasPrivateKey
```

## 构建并验证

从干净工作区运行：

```powershell
npm run release:build:public
```

该命令会在构建前拒绝被 Git 跟踪的签名配置、秘密字段、非法 thumbprint、非 SHA-256 摘要、缺失/过期/无私钥证书。Tauri 在创建 NSIS 前签应用，并生成已签名安装包；随后严格验证器再次要求干净源码和有效 Authenticode 签名。

最终输出必须同时包含：

```text
THIRD_PARTY_NOTICES_OK
ARTIFACT_SCAN_CLEAN
SourceClean : True
Signature   : Valid
```

把 `release-manifest.json`、它的控制台 SHA-256、安装包和 EXE 作为同一组发布证据归档。不要提交本地签名配置、PFX、密码或私钥。
