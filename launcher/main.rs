#![cfg(windows)]
#![windows_subsystem = "windows"]

//! 启动器外壳：拉起 node 跑管理服务，再把这个管理页装进一个自己的窗口。
//!
//! 之所以要窗口而不是丢给系统浏览器：浏览器只允许脚本关闭「自己用 window.open 打开的」
//! 页面，所以系统浏览器里那个标签页谁也关不掉——托盘退出时只能指望页面自己 window.close()，
//! 关不掉还得退化成一张告别页。窗口是本进程创建的，收放就都是自己的事。
//!
//! 关窗口 ≠ 退出：托盘还在、dsh 照常跑；托盘点「打开管理页」时 start.js 会打 SHOW_PORT，
//! 把隐藏的窗口叫回来。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tao::dpi::{LogicalSize, PhysicalPosition};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use wry::{WebContext, WebViewBuilder};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MANAGER_URL: &str = "http://127.0.0.1:3780/";
const MANAGER_ADDR: &str = "127.0.0.1:3780";
/// 等管理服务起来的时限；超了也照常开窗口，让页面自己显示连接失败。
const PORT_WAIT: Duration = Duration::from_secs(25);
/// 首次出现的窗口尺寸（逻辑像素）
const WINDOW_W: f64 = 1100.0;
const WINDOW_H: f64 = 760.0;
const WINDOW_MIN_W: f64 = 720.0;
const WINDOW_MIN_H: f64 = 520.0;
/// node 往 stdout 打这一行，就表示它要我们把窗口叫到前面（见 start.js 的 requestShow）。
const SHOW_SIGNAL: &str = "__DSH_SHOW__";
/// 菜单项 id
const ITEM_OPEN_DSH: &str = "open-dsh";
const ITEM_TOGGLE: &str = "toggle";
const ITEM_RESTART: &str = "restart";
const ITEM_MANAGER: &str = "manager";
const ITEM_QUIT: &str = "quit";

#[derive(Debug)]
enum UserEvent {
    Show,
    Exited,
    /// 以下几个来自页面：原生标题栏去掉了，最小化/最大化/关闭/拖动都由页面发过来
    Minimize,
    ToggleMaximize,
    Hide,
    Drag,
    /// 轮询到的最新托盘状态
    Tray(TrayState),
}

/// 从 /api/tray 读回来的状态（纯文本 key=value，见 server.js）
#[derive(Debug, Default, Clone)]
struct TrayState {
    status: String,
    url: String,
    installed: bool,
}

impl TrayState {
    fn live(&self) -> bool {
        self.status == "running" || self.status == "starting"
    }
}

fn parse_tray_state(text: &str) -> TrayState {
    let mut state = TrayState::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "status" => state.status = value.trim().to_string(),
            "url" => state.url = value.trim().to_string(),
            "installed" => state.installed = value.trim() == "1",
            _ => {}
        }
    }
    state
}

/// 原生托盘。菜单项句柄要留着——状态一变就得改文案和可用性。
struct Tray {
    _icon: TrayIcon,
    open_dsh: MenuItem,
    toggle: MenuItem,
    restart: MenuItem,
}

impl Tray {
    fn sync(&self, state: &TrayState) {
        let live = state.live();
        self.open_dsh.set_enabled(!state.url.is_empty());
        self.toggle.set_text(if live { "停止" } else { "启动" });
        self.toggle.set_enabled(state.installed && state.status != "stopping");
        self.restart.set_enabled(state.status == "running");
    }
}

/// 托盘图标用不了 256 那帧（托盘只要 16/32），从大图缩下去。
fn load_tray_icon(root: &Path) -> Option<tray_icon::Icon> {
    let bytes = std::fs::read(root.join("assets").join("tray.ico")).ok()?;
    let (rgba, width, height) = decode_ico(&bytes)?;
    let source = image::RgbaImage::from_raw(width, height, rgba)?;
    let scaled = image::imageops::resize(&source, 32, 32, image::imageops::FilterType::Lanczos3);
    tray_icon::Icon::from_rgba(scaled.into_raw(), 32, 32).ok()
}

fn build_tray(root: &Path) -> Option<Tray> {
    let menu = Menu::new();
    let open_dsh = MenuItem::with_id(ITEM_OPEN_DSH, "打开 DSH", false, None);
    let toggle = MenuItem::with_id(ITEM_TOGGLE, "启动", false, None);
    let restart = MenuItem::with_id(ITEM_RESTART, "重启 DSH", false, None);
    let separator = PredefinedMenuItem::separator();
    let manager = MenuItem::with_id(ITEM_MANAGER, "打开管理页", true, None);
    let quit = MenuItem::with_id(ITEM_QUIT, "退出", true, None);
    for item in [
        &open_dsh as &dyn tray_icon::menu::IsMenuItem,
        &toggle,
        &restart,
        &separator,
        &manager,
        &quit,
    ] {
        let _ = menu.append(item);
    }
    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        // 左键留给「打开启动器界面」，菜单走右键
        .with_menu_on_left_click(false)
        .with_tooltip("DSH-X")
        .with_icon(load_tray_icon(root)?)
        .build()
        .ok()?;
    Some(Tray {
        _icon: icon,
        open_dsh,
        toggle,
        restart,
    })
}

fn alert(message: &str) {
    let _ = Command::new("mshta")
        .arg(format!("javascript:alert('{message}');close()"))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn manager_is_up() -> bool {
    MANAGER_ADDR
        .parse()
        .ok()
        .and_then(|addr: std::net::SocketAddr| TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok())
        .is_some()
}

/// 拉不起窗口时退回老做法：让系统浏览器打开管理页。
fn open_in_browser(url: &str) {
    let _ = Command::new("cmd")
        .args(["/c", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

/// 极简 HTTP GET，只用来问本机管理服务一个短路径；读完整响应取正文即可。
fn http_get(addr: &str, path: &str) -> Option<String> {    let mut stream = TcpStream::connect(addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes())
        .ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    Some(
        raw.split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or_default()
            .trim()
            .to_string(),
    )
}

/// 同样极简的 POST，用来让本机管理服务执行托盘菜单的动作；正文固定给个空 JSON。
fn http_post(addr: &str, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).ok()?;
    stream.write_all(b"{}").ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    Some(raw)
}

/// 把 dsh.ico 解成 RGBA。ico 里 256 那帧是 PNG、小帧是 BMP，交给 image 统一处理。
fn decode_ico(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Ico).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((rgba.into_raw(), width, height))
}

/// 窗口图标。exe 里内嵌的是同一张图，但 tao 不会自动取用，得自己解出来给它。
fn load_window_icon(root: &Path) -> Option<tao::window::Icon> {
    let bytes = std::fs::read(root.join("assets").join("dsh.ico")).ok()?;
    let (rgba, width, height) = decode_ico(&bytes)?;
    tao::window::Icon::from_rgba(rgba, width, height).ok()
}

fn main() {
    let exe = std::env::current_exe().expect("current exe");
    let root = exe.parent().expect("install dir").to_path_buf();
    let node = root.join("node").join("node.exe");
    let script = root.join("start.js");
    if !node.exists() || !script.exists() {
        alert("DSH 启动失败：缺少 node/node.exe 或 start.js");
        std::process::exit(1);
    }

    // 已经有实例在跑就别再走后面那一套了。否则会先建出一个窗口、再拉一次 node 和
    // WebView2，等发现端口被占才收摊——用户看到的就是一个多余的窗口闪一下。
    // 直接让那个实例把窗口叫出来就完事（它的 node 收到 /api/wake 会回信号给我们）。
    if manager_is_up() {
        let _ = http_post(MANAGER_ADDR, "/api/wake");
        std::process::exit(0);
    }

    let mut path = node.parent().unwrap().display().to_string();
    if let Ok(old) = std::env::var("PATH") {
        path.push(';');
        path.push_str(&old);
    }

    let spawn_node = |app_window: bool| {
        let mut command = Command::new(&node);
        command
            .arg(&script)
            .current_dir(&root)
            .env("PATH", &path)
            // stdout 走管道：node 用一行约定标记叫我们把窗口叫到前面
            .stdout(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        if app_window {
            // 告诉 start.js：管理页由本进程的窗口承载，别再自己开浏览器
            command.env("DSH_APP_WINDOW", "1");
        }
        command.spawn()
    };

    let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
    let event_loop = builder.build();
    let proxy = event_loop.create_proxy();

    // 先把 node 拉起来：它要加载模块、起 http 服务，这段时间正好和下面 WebView2 的初始化重叠
    let mut child = match spawn_node(true) {
        Ok(child) => child,
        Err(error) => {
            alert(&format!("DSH 启动失败：{error}"));
            std::process::exit(1);
        }
    };

    let node_out = child.stdout.take();

    // node 退出（含托盘退出）就收摊，窗口跟着关
    {
        let proxy = proxy.clone();
        thread::spawn(move || {
            let _ = child.wait();
            let _ = proxy.send_event(UserEvent::Exited);
        });
    }

    // node 说「把窗口叫出来」时，读的就是这句约定标记
    if let Some(out) = node_out {
        let proxy = proxy.clone();
        thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if line.trim() == SHOW_SIGNAL {
                    let _ = proxy.send_event(UserEvent::Show);
                }
            }
        });
    }

    // 每 1.5 秒问一次状态，用来更新托盘菜单的文案和可用项
    {
        let proxy = proxy.clone();
        thread::spawn(move || loop {
            let state = http_get(MANAGER_ADDR, "/api/tray")
                .map(|text| parse_tray_state(&text))
                .unwrap_or_default();
            if proxy.send_event(UserEvent::Tray(state)).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(1500));
        });
    }

    // WebView2 的初始化是启动里最贵的一段，放在 node 之后做，两者就能并行。
    // 建不出来（WebView2 缺失等）就降级成「只有托盘、没有窗口」，浏览器顶上管理页，
    // 绝不能在 node 已经在跑的时候直接退出——那会留下没人管的进程。
    let data_dir = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.clone())
        .join("DSH")
        .join("webview");
    let _ = std::fs::create_dir_all(&data_dir);
    let mut context = WebContext::new(Some(data_dir));

    // 无边框窗口拿不到系统给的默认摆位，不指定的话会落在系统的层叠位置上；首次出现摆在主屏正中
    let centered = event_loop.primary_monitor().map(|monitor| {
        let screen = monitor.size();
        let area = LogicalSize::new(WINDOW_W, WINDOW_H).to_physical::<u32>(monitor.scale_factor());
        PhysicalPosition::new(
            monitor.position().x + (screen.width as i32 - area.width as i32) / 2,
            monitor.position().y + (screen.height as i32 - area.height as i32) / 2,
        )
    });

    let mut window_builder = WindowBuilder::new()
        .with_title("DSH-X")
        .with_window_icon(load_window_icon(&root))
        // 不要原生标题栏：图标、标题、三个按钮都由页面自己画，风格才统一
        .with_decorations(false)
        .with_inner_size(LogicalSize::new(WINDOW_W, WINDOW_H))
        .with_min_inner_size(LogicalSize::new(WINDOW_MIN_W, WINDOW_MIN_H));
    if let Some(position) = centered {
        window_builder = window_builder.with_position(position);
    }
    let window = match window_builder
        .build(&event_loop)
    {
        Ok(window) => Some(window),
        Err(error) => {
            alert(&format!("DSH-X 窗口创建失败，已改用浏览器打开：{error}"));
            open_in_browser(MANAGER_URL);
            None
        }
    };

    // 页面的窗口操作走 IPC 转成事件，窗口本身留在事件循环里操作，避免到处共享所有权
    let ipc_proxy = proxy.clone();
    let webview = match &window {
        Some(window) => match WebViewBuilder::new_with_web_context(&mut context)
            .with_url("about:blank")
            .with_ipc_handler(move |request| {
                let event = match request.body().as_str() {
                    "minimize" => UserEvent::Minimize,
                    "maximize" => UserEvent::ToggleMaximize,
                    "close" => UserEvent::Hide,
                    "drag" => UserEvent::Drag,
                    _ => return,
                };
                let _ = ipc_proxy.send_event(event);
            })
            .build(window)
        {
            Ok(webview) => Some(webview),
            Err(error) => {
                alert(&format!("DSH-X 窗口创建失败，已改用浏览器打开：{error}"));
                open_in_browser(MANAGER_URL);
                None
            }
        },
        None => None,
    };

    // 等管理服务起来再导航，否则 webview 会先撞上连接失败（它自己不会重试）
    let deadline = Instant::now() + PORT_WAIT;
    while Instant::now() < deadline && !manager_is_up() {
        thread::sleep(Duration::from_millis(200));
    }

    // 直接加载，不问服务端：window=1 是个常量，绕一趟 HTTP 只会拖慢启动——而且
    // 那个接口内部要发网络请求，一旦超过读超时就会退化成不带 window=1 的地址，
    // 页面因此丢掉自定义标题栏和窗口按钮。更新询问交给页面自己去问 /api/pending。
    if let Some(webview) = &webview {
        let _ = webview.load_url(&format!("{MANAGER_URL}?window=1"));
    }

    // 托盘建在主线程：它的消息要靠下面这个事件循环的消息泵派发
    let tray = build_tray(&root);
    let mut state = TrayState::default();

    // webview 与 context 都是本帧的局部变量，run 不返回，所以它们的生命周期覆盖整个窗口期
    event_loop.run(move |event, _, control_flow| {
        // 左键点托盘＝打开启动器界面（菜单已经改成只在右键弹）
        while let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = tray_event
            {
                match &window {
                    Some(window) => {
                        // 被 ─ 收到任务栏的窗口，光 set_visible + set_focus 拉不回来
                        window.set_minimized(false);
                        window.set_visible(true);
                        window.set_focus();
                    }
                    None => open_in_browser(MANAGER_URL),
                }
            }
        }
        // 菜单事件走 muda 的全局 channel；托盘和窗口同在这条线程上，直接收
        while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            match menu_event.id().as_ref() {
                ITEM_OPEN_DSH => {
                    if !state.url.is_empty() {
                        open_in_browser(&state.url);
                    }
                }
                ITEM_TOGGLE => {
                    let path = if state.live() { "/api/stop" } else { "/api/launch" };
                    let _ = http_post(MANAGER_ADDR, path);
                }
                ITEM_RESTART => {
                    let _ = http_post(MANAGER_ADDR, "/api/restart");
                }
                ITEM_MANAGER => match &window {
                    Some(window) => {
                        // 被 ─ 收到任务栏的窗口，光 set_visible + set_focus 拉不回来
                        window.set_minimized(false);
                        window.set_visible(true);
                        window.set_focus();
                    }
                    None => open_in_browser(MANAGER_URL),
                },
                ITEM_QUIT => {
                    // 让 node 自己收尾（停掉 dsh、通知开着的页面），它一退我们跟着收摊
                    let _ = http_post(MANAGER_ADDR, "/api/quit");
                }
                _ => {}
            }
        }
        // 菜单事件不会唤醒事件循环，所以按小步长醒着轮询
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(80));
        match event {
            Event::UserEvent(UserEvent::Show) => {
                if let Some(window) = &window {
                    window.set_minimized(false);
                    window.set_visible(true);
                    window.set_focus();
                }
            }
            Event::UserEvent(UserEvent::Exited) => *control_flow = ControlFlow::ExitWithCode(0),
            Event::UserEvent(UserEvent::Tray(next)) => {
                state = next;
                if let Some(tray) = &tray {
                    tray.sync(&state);
                }
            }
            Event::UserEvent(UserEvent::Minimize) => {
                if let Some(window) = &window {
                    window.set_minimized(true);
                }
            }
            Event::UserEvent(UserEvent::ToggleMaximize) => {
                if let Some(window) = &window {
                    window.set_maximized(!window.is_maximized());
                }
            }
            Event::UserEvent(UserEvent::Hide) => {
                if let Some(window) = &window {
                    window.set_visible(false);
                }
            }
            Event::UserEvent(UserEvent::Drag) => {
                if let Some(window) = &window {
                    let _ = window.drag_window();
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // 关窗口不退出：托盘还在、dsh 照常跑，需要时从托盘再叫回来
                if let Some(window) = &window {
                    window.set_visible(false);
                }
            }
            _ => {}
        }
        let _ = &webview;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_icon_at_full_size() {
        let bytes = std::fs::read("../assets/dsh.ico").expect("read assets/dsh.ico");
        let (rgba, width, height) = decode_ico(&bytes).expect("decode ico");
        println!("icon decoded {width}x{height}, {} bytes", rgba.len());
        assert_eq!(rgba.len() as u32, width * height * 4, "RGBA buffer size");
        // image 该挑最大那帧；挑到 16x16 标题栏图标就糊了
        assert!(width >= 256 && height >= 256, "expected the 256 frame, got {width}x{height}");
    }

    #[test]
    fn missing_icon_is_not_fatal() {
        assert!(load_window_icon(Path::new("does-not-exist")).is_none());
    }
}
