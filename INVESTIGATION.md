# GigaFile/gfile Rust rewrite investigation

调查时间：2026-07-03（Asia/Singapore）

本文档用于给后续方案制定者提供背景和事实输入。它只记录背景、协议观察、许可/条款约束和 Rust 价值判断，不包含具体实现路线。

## 背景

[GigaFile 便](https://gigafile.nu/) 是日本的大文件传输服务。官网说明它无需用户注册即可使用，支持单文件最大 300GB，上传后的文件最多保留 100 天。服务面向普通网页上传/下载场景，也有广告和付费去广告会员功能。

[fireattack/gfile](https://github.com/fireattack/gfile) 是一个 Python CLI/module，用于从 gigafile.nu 上传和下载文件。PyPI 包名是 `gigafile`，但命令和模块名仍是 `gfile`。项目 README 展示的命令包括：

- `gfile upload path/to/file`
- `gfile download https://XX.gigafile.nu/...`
- 可选 `--aria2` 使用 aria2 下载
- 上传相关参数包括线程数、chunk size、copy size、timeout
- 下载可传 `--key/--password`

仓库当前 README 页面显示 license 为 GPL-3.0。源码中的 `gfile/__init__.py` 版本号为 `3.2.5`；GitHub release 页面曾显示 `3.2.4` 为 latest release，但远端 tag 中存在 `3.2.5`。后续若需要锁定兼容目标，应以具体 commit/tag 为准。

## 现有 Python 实现观察

源码主体位于 `gfile/gfile.py`，核心类为 `GFile`。

下载行为：

- 先请求下载页 URL，用 BeautifulSoup 解析页面。
- URL 形态校验大致为 `https://<number>.gigafile.nu/<id>`。
- 普通单文件页面从 `#dl` 取文件名，从 `.dl_size` 取大小，file id 来自 URL path。
- `matomete` 多文件页面通过 `#contents_matomete`、`.matomete_file` 和下载按钮 `onclick` 提取多个文件信息。
- 实际下载 URL 形态为 `https://<server>.gigafile.nu/download.php?file=<file_id>`。
- 如果提供密码，会追加 `&dlkey=<key>`。
- Python 内置下载是单连接流式写入临时文件，完成后用 `Content-Length` 和本地文件大小做检查，再重命名。
- 如果使用 `--aria2`，会把当前 session cookies 作为 `Cookie` header 传给 `aria2c`。

上传行为：

- 先访问 `https://gigafile.nu/`，从页面 JavaScript 中解析 `var server = "...";`。
- 上传 endpoint 为 `https://<server>/upload_chunk.php`。
- 每个 chunk 使用 multipart/form-data，字段包括 `id`、`name`、`chunk`、`chunks`、`lifetime`、`file`。
- `id` 当前由 `uuid.uuid1().hex` 生成。
- `lifetime` 当前固定为 `100`。
- 第一块先上传，用于建立 cookie/session；之后用线程池上传剩余块。
- 代码刻意让各 chunk 的请求结束顺序与 chunk 序号一致，以避免最终文件损坏。
- 上传完成后从响应 JSON 中读取 `url`，并可通过下载 URL 的 `Content-Length` 验证上传文件大小。

一个值得关注的性能点：Python 版在上传 chunk 时先把 multipart encoder 转成完整 bytes，再流式喂给 requests。这意味着每个并发 chunk 至少会占用接近 `chunk_size + multipart overhead` 的内存，默认 README 示例为 `100MB * thread_num` 级别。

## 许可与合规判断

`fireattack/gfile` 仓库标记为 GPL-3.0，原始 fork 仓库也标记为 GPL-3.0。若 Rust 版直接基于当前 Python 源码、结构、解析规则和字段逻辑重写并发布，保守处理应当：

- 采用 GPL-3.0 兼容发布方式。
- 保留原项目版权和许可证说明。
- 明确说明 Rust 版受 `fireattack/gfile` / `Sraq-Zit/gfile` 启发或派生。
- 发布二进制时同步提供完整对应源码。

如果要做非 GPL 许可，必须走更严格的 clean-room：一方只写协议规格，另一方不读原源码独立实现。但当前调查已经读过原源码，后续继续参考这些细节时，不建议把项目包装成完全独立来源。

GigaFile 服务条款方面，[官方利用规约/隐私页面](https://gigafile.nu/privacy.php) 没有明确禁止命令行客户端，但有几类约束需要遵守：

- 禁止违法、公序良俗违反、犯罪相关、侵权或妨害服务运营的文件与行为。
- 禁止不正访问或尝试不正访问。
- 禁止给服务器或线路造成通常利用以外负载的行为。
- 条款特别提到，预计会带来大量访问的掲示板等场所张贴特定文件下载 URL，以及通常利用以外的服务器/线路负载，属于妨害运营风险。
- 邮件通知等涉及邮箱输入的功能要求使用用户自己的邮箱。

因此，一个合理的 Rust CLI 应定位为正常网页操作的自动化辅助，而不是绕过限制、批量扫描、密码尝试、刷下载或高并发压测工具。默认行为应避免让用户无意识地产生异常负载。

## Git/GitHub 要求

目标仓库已创建：

- GitHub: https://github.com/Maymall/gfile-rust
- Visibility: public
- 建议默认分支：`main`

后续项目应直接以该仓库为发布目标。Git 使用要求：

- 仓库内必须保留 GPL-3.0 许可证文本，并在 README 或 NOTICE 中说明与 `fireattack/gfile` / `Sraq-Zit/gfile` 的关系。
- 不要提交真实下载链接、真实密码、cookie、token、GitHub 凭据、测试账号或任何私有文件。
- 不要把实际下载产物、上传测试文件、临时文件、构建产物或大体积 fixture 提交进仓库。
- 示例 URL 使用明显的假数据，或使用专门标记为测试/已失效的样例。
- commit 应保持小而可审查，信息说明用户可见行为或工程约束，例如 `docs: add protocol investigation`、`feat: add download page parser`。
- 避免 force push 覆盖共享历史；确需改历史时先确认。
- 版本发布使用 git tag，并确保 tag 对应源码、二进制产物和 changelog 一致。
- 若发布二进制，release 页面必须给出对应源码获取方式，符合 GPL-3.0 对源代码分发的要求。
- CI、测试数据和文档变更都应进入版本控制；本地环境专用配置应放入 ignored 文件或文档化的示例模板。

## Rust 可能带来的收益

下载吞吐不应过度承诺。GigaFile 下载通常受用户网络、服务端线路、单连接限速、文件所在节点和是否支持 range 请求影响。单纯从 Python 换成 Rust，在单连接 HTTP 流式下载上通常不会显著快于现有实现。当前 Python 版还支持 `aria2`，如果服务器支持多连接 range 下载，`aria2` 可能已经接近下载性能上限。

Rust 更确定的收益在这些方面：

- 分发体验：单个原生二进制，不要求用户安装 Python、pip、requests、bs4、tqdm 等运行时依赖。
- 启动和资源占用：CLI 启动更快，常驻内存更低，适合脚本和批处理场景。
- 上传内存控制：可以实现真正的 streaming multipart，避免把每个 chunk 的完整 multipart body 先放入内存。
- 并发模型：异步 HTTP 或受控线程模型更容易精细控制并发、取消、超时和进度汇报。
- 错误处理：Rust 类型系统适合把页面解析失败、HTTP 失败、密码错误、文件过期、大小不匹配、临时文件冲突等错误做成清晰的枚举。
- 稳定性：断点续传、临时文件恢复、原子重命名、下载校验等行为可以做得比当前脚本更系统。

需要注意，Python 的下载和上传大多是 I/O bound，requests 在线程上传时也不会单纯受 GIL 限制。因此 Rust 的优势主要是可靠性、内存、分发和可维护性，而不是 CPU 性能。

## 协议风险与未知项

以下内容需要后续在真实样本上确认，但本文档不制定处理路线：

- 下载 endpoint 是否稳定，是否仍长期使用 `/download.php?file=...`。
- 下载是否支持 HTTP Range，以及多连接下载是否会触发限速或异常。
- 密码错误、文件过期、文件不存在、病毒/危险文件拦截时页面和 HTTP 状态如何表现。
- `matomete` 多文件页面结构是否稳定，文件名中日文、emoji、特殊符号、Windows 禁用字符如何处理。
- 上传接口是否要求特定 cookie、referer、user-agent 或其他隐藏字段。
- `upload_chunk.php` 响应 JSON 的完整 schema，以及 `status` 字段语义。
- chunk 完成顺序为何影响最终文件完整性，服务端是否按完成顺序拼接，还是存在其他时序依赖。
- 上传 lifetime 是否只允许官网列出的保留期限值。
- 付费去广告会员场景是否有不同下载页面或 cookie 行为。

## 给后续方案制定者的输入

这不是一个破解协议项目，而是把网页公开上传/下载行为做成 CLI。后续设计应以“保守、可恢复、低负载、可解释错误”为基准。Rust 版最有价值的卖点不是绝对下载速度，而是大文件上传/下载时的资源控制、可靠性和无运行时依赖分发。

建议后续任何方案都显式写清：

- 许可证选择和与 GPL-3.0 上游的关系。
- 默认并发为何不会构成异常负载。
- 与 aria2 的关系：替代、调用、还是不覆盖。
- 失败时是否留下可恢复状态和可诊断日志。
- 是否只支持下载，还是同时覆盖上传。

## 参考链接

- fireattack/gfile: https://github.com/fireattack/gfile
- Sraq-Zit/gfile: https://github.com/Sraq-Zit/gfile
- GigaFile 官网: https://gigafile.nu/
- GigaFile 利用规约/隐私政策: https://gigafile.nu/privacy.php
- GPL-3.0 概览: https://choosealicense.com/licenses/gpl-3.0/
