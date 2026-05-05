use std::{
    env, fs, io,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use chrono::{DateTime, FixedOffset, LocalResult, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{CrosstermBackend, Frame, Terminal},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use tui_big_text::{BigText, PixelSize};

const TITLE_ART: &[&str] = &[
    " ___                          ",
    "  |  _  ._ _|_ o ._ _   _  ._ ",
    "  | (_) |_) |_ | | | | (/_ |  ",
    "        |                     ",
];

const COMMON_TIMEZONES: &[(&str, &str)] = &[
    ("AOE / Anywhere on Earth", "Etc/GMT+12"),
    ("UTC", "UTC"),
    ("中国 / Beijing", "Asia/Shanghai"),
    ("日本 / Tokyo", "Asia/Tokyo"),
    ("美国东部 / US Eastern", "America/New_York"),
    ("美国中部 / US Central", "America/Chicago"),
    ("美国山区 / US Mountain", "America/Denver"),
    ("美国西部 / US Pacific", "America/Los_Angeles"),
    ("英国 / London", "Europe/London"),
    ("欧洲中部 / Central Europe", "Europe/Paris"),
];

const DIGIT_FONT_NAMES: &[&str] = &["tube", "block", "compact"];

const PRECISIONS: &[(&str, &str)] = &[
    ("minutes", "分钟 HH:MM"),
    ("seconds", "秒 HH:MM:SS"),
    ("tenths", "十分之一秒 HH:MM:SS.d"),
    ("hundredths", "百分之一秒 HH:MM:SS.dd"),
    ("milliseconds", "毫秒 HH:MM:SS.ddd"),
];

const LANGUAGES: &[(&str, &str)] = &[("zh", "中文"), ("ja", "日本語"), ("en", "English")];

fn default_language() -> String {
    "zh".to_string()
}

#[derive(Clone, Copy)]
enum Lang {
    Zh,
    Ja,
    En,
}

fn lang_from_code(code: &str) -> Lang {
    match code {
        "ja" => Lang::Ja,
        "en" => Lang::En,
        _ => Lang::Zh,
    }
}

fn language_label(code: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|(candidate, _)| *candidate == code)
        .map(|(_, label)| *label)
        .unwrap_or("中文")
}

fn tr<'a>(lang: Lang, key: &'a str) -> &'a str {
    match (lang, key) {
        (Lang::Zh, "menu_start") => "开始计时",
        (Lang::Zh, "menu_add") => "添加计时",
        (Lang::Zh, "menu_settings") => "设置",
        (Lang::Zh, "menu_config") => "查看配置",
        (Lang::Zh, "menu_quit") => "退出",
        (Lang::Zh, "menu_help") => "↑/↓ 选择 · Enter 确认 · q 退出",
        (Lang::Zh, "config") => "配置",
        (Lang::Zh, "home_tz") => "主时区",
        (Lang::Zh, "precision") => "精度",
        (Lang::Zh, "no_timers") => {
            "还没有计时。

返回主菜单选择“添加计时”。

q 返回"
        }
        (Lang::Zh, "timer_list") => "计时列表",
        (Lang::Zh, "finished") => "已结束",
        (Lang::Zh, "invalid_time") => "时间无效",
        (Lang::Zh, "distance") => "距离目标",
        (Lang::Zh, "target") => "目标",
        (Lang::Zh, "timer_footer") => "q 返回 · r 重新读取配置 · ←/→ 字体: {font} · ↑/↓ 计时",
        (Lang::Zh, "add_title") => "标题",
        (Lang::Zh, "add_target") => "目标时间",
        (Lang::Zh, "timezone") => "时区",
        (Lang::Zh, "note") => "备注",
        (Lang::Zh, "save") => "保存",
        (Lang::Zh, "add_hint_time") => "时间格式: YYYY-MM-DD HH:MM，例如 2026-05-05 23:59",
        (Lang::Zh, "add_hint_tz") => "在时区行用 ←/→ 选择 AOE、UTC、中国、美东、美西等预设。",
        (Lang::Zh, "add_footer") => "Tab/↑/↓ 切换 · Enter 下一项/保存 · Esc 返回",
        (Lang::Zh, "added") => "已添加计时",
        (Lang::Zh, "settings_hint") => {
            "用 ←/→ 修改；保存会立即写入 ~/.config/toptimer/config.json。"
        }
        (Lang::Zh, "default_add_tz") => "默认添加时区",
        (Lang::Zh, "language") => "语言",
        (Lang::Zh, "settings_footer") => "q/Esc 返回",
        (Lang::Zh, "config_title") => "配置",

        (Lang::Ja, "menu_start") => "タイマー開始",
        (Lang::Ja, "menu_add") => "タイマー追加",
        (Lang::Ja, "menu_settings") => "設定",
        (Lang::Ja, "menu_config") => "設定ファイル表示",
        (Lang::Ja, "menu_quit") => "終了",
        (Lang::Ja, "menu_help") => "↑/↓ 選択 · Enter 決定 · q 終了",
        (Lang::Ja, "config") => "設定",
        (Lang::Ja, "home_tz") => "メインタイムゾーン",
        (Lang::Ja, "precision") => "表示精度",
        (Lang::Ja, "no_timers") => {
            "タイマーがありません。

メインメニューから「タイマー追加」を選んでください。

q 戻る"
        }
        (Lang::Ja, "timer_list") => "タイマー一覧",
        (Lang::Ja, "finished") => "終了",
        (Lang::Ja, "invalid_time") => "時刻が無効",
        (Lang::Ja, "distance") => "目標まで",
        (Lang::Ja, "target") => "目標",
        (Lang::Ja, "timer_footer") => "q 戻る · r 再読み込み · ←/→ フォント: {font} · ↑/↓ タイマー",
        (Lang::Ja, "add_title") => "タイトル",
        (Lang::Ja, "add_target") => "目標時刻",
        (Lang::Ja, "timezone") => "タイムゾーン",
        (Lang::Ja, "note") => "メモ",
        (Lang::Ja, "save") => "保存",
        (Lang::Ja, "add_hint_time") => "形式: YYYY-MM-DD HH:MM、例 2026-05-05 23:59",
        (Lang::Ja, "add_hint_tz") => {
            "タイムゾーン行で ←/→ を使って AOE、UTC、中国、米国タイムゾーンなどを選択。"
        }
        (Lang::Ja, "add_footer") => "Tab/↑/↓ 移動 · Enter 次/保存 · Esc 戻る",
        (Lang::Ja, "added") => "タイマーを追加しました",
        (Lang::Ja, "settings_hint") => {
            "←/→ で変更。~/.config/toptimer/config.json に保存されます。"
        }
        (Lang::Ja, "default_add_tz") => "追加時の既定タイムゾーン",
        (Lang::Ja, "language") => "言語",
        (Lang::Ja, "settings_footer") => "q/Esc 戻る",
        (Lang::Ja, "config_title") => "設定",

        (Lang::En, "menu_start") => "Start timer",
        (Lang::En, "menu_add") => "Add timer",
        (Lang::En, "menu_settings") => "Settings",
        (Lang::En, "menu_config") => "View config",
        (Lang::En, "menu_quit") => "Quit",
        (Lang::En, "menu_help") => "↑/↓ select · Enter confirm · q quit",
        (Lang::En, "config") => "Config",
        (Lang::En, "home_tz") => "Home timezone",
        (Lang::En, "precision") => "Precision",
        (Lang::En, "no_timers") => {
            "No timers yet.

Choose Add timer from the main menu.

q back"
        }
        (Lang::En, "timer_list") => "Timers",
        (Lang::En, "finished") => "Finished",
        (Lang::En, "invalid_time") => "Invalid time",
        (Lang::En, "distance") => "Distance to target",
        (Lang::En, "target") => "Target",
        (Lang::En, "timer_footer") => "q back · r reload config · ←/→ font: {font} · ↑/↓ timer",
        (Lang::En, "add_title") => "Title",
        (Lang::En, "add_target") => "Target time",
        (Lang::En, "timezone") => "Timezone",
        (Lang::En, "note") => "Note",
        (Lang::En, "save") => "Save",
        (Lang::En, "add_hint_time") => "Time format: YYYY-MM-DD HH:MM, e.g. 2026-05-05 23:59",
        (Lang::En, "add_hint_tz") => {
            "Use ←/→ on the timezone row to choose AOE, UTC, China, US timezones, and more."
        }
        (Lang::En, "add_footer") => "Tab/↑/↓ move · Enter next/save · Esc back",
        (Lang::En, "added") => "Timer added",
        (Lang::En, "settings_hint") => {
            "Use ←/→ to change; saved to ~/.config/toptimer/config.json immediately."
        }
        (Lang::En, "default_add_tz") => "Default add timezone",
        (Lang::En, "language") => "Language",
        (Lang::En, "settings_footer") => "q/Esc back",
        (Lang::En, "config_title") => "Config",
        _ => key,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    version: u8,
    settings: Settings,
    timezone_presets: Vec<TimezonePreset>,
    timers: Vec<TimerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    home_timezone: String,
    display_precision: String,
    default_add_timezone: String,
    #[serde(default = "default_language")]
    language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimezonePreset {
    label: String,
    zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimerConfig {
    id: String,
    title: String,
    target: String,
    timezone: String,
    note: String,
    created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    TimerList,
    TimerRun,
    Add,
    Settings,
    ConfigView,
}

struct App {
    config: Config,
    screen: Screen,
    menu_index: usize,
    timer_index: usize,
    settings_index: usize,
    add_field: usize,
    add_title: String,
    add_target: String,
    add_note: String,
    add_tz_index: usize,
    digit_font_index: usize,
    message: String,
    should_quit: bool,
}

impl App {
    fn new(config: Config) -> Self {
        let add_tz_index = preset_index_config(&config, &config.settings.default_add_timezone);
        Self {
            config,
            screen: Screen::Menu,
            menu_index: 0,
            timer_index: 0,
            settings_index: 0,
            add_field: 0,
            add_title: String::new(),
            add_target: String::new(),
            add_note: String::new(),
            add_tz_index,
            digit_font_index: 0,
            message: String::new(),
            should_quit: false,
        }
    }

    fn selected_timer(&self) -> Option<&TimerConfig> {
        self.config.timers.get(self.timer_index)
    }

    fn digit_font(&self) -> DigitFont {
        match self.digit_font_index % DIGIT_FONT_NAMES.len() {
            0 => DigitFont::Tube,
            1 => DigitFont::Block,
            _ => DigitFont::Compact,
        }
    }

    fn digit_font_name(&self) -> &'static str {
        DIGIT_FONT_NAMES[self.digit_font_index % DIGIT_FONT_NAMES.len()]
    }

    fn lang(&self) -> Lang {
        lang_from_code(&self.config.settings.language)
    }
}

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if matches!(args.first().map(String::as_str), Some("init")) {
        save_config(&default_config())?;
        println!("已初始化配置: {}", config_path().display());
        return Ok(());
    }
    if matches!(args.first().map(String::as_str), Some("config")) {
        ensure_config()?;
        println!("{}", config_path().display());
        return Ok(());
    }
    if matches!(args.first().map(String::as_str), Some("fonts")) {
        for font in DIGIT_FONT_NAMES {
            println!("{font}");
        }
        return Ok(());
    }
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        println!("toptimer\n\n用法:\n  toptimer        启动 TUI\n  toptimer init   初始化配置\n  toptimer config 输出配置路径
  toptimer fonts  输出可切换数字字体");
        return Ok(());
    }

    let config = ensure_config()?;
    run_tui(App::new(config))
}

fn run_tui(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = app_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app))?;
        let timeout = refresh_interval(&app.config).saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let CEvent::Key(key) = event::read()? {
                handle_key(app, key)?;
            }
        }
        if last_tick.elapsed() >= refresh_interval(&app.config) {
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match app.screen {
        Screen::Menu => handle_menu_key(app, key),
        Screen::TimerList => handle_timer_list_key(app, key),
        Screen::TimerRun => handle_timer_run_key(app, key),
        Screen::Add => handle_add_key(app, key),
        Screen::Settings => handle_settings_key(app, key),
        Screen::ConfigView => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => app.screen = Screen::Menu,
                _ => {}
            }
            Ok(())
        }
    }
}

fn handle_menu_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.menu_index = app.menu_index.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => app.menu_index = (app.menu_index + 1).min(4),
        KeyCode::Enter => match app.menu_index {
            0 => app.screen = Screen::TimerList,
            1 => app.screen = Screen::Add,
            2 => app.screen = Screen::Settings,
            3 => app.screen = Screen::ConfigView,
            4 => app.should_quit = true,
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn handle_timer_list_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Menu,
        KeyCode::Char('r') => app.config = ensure_config()?,
        KeyCode::Up | KeyCode::Char('k') => app.timer_index = app.timer_index.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.config.timers.is_empty() {
                app.timer_index = (app.timer_index + 1).min(app.config.timers.len() - 1);
            }
        }
        KeyCode::Enter => {
            if !app.config.timers.is_empty() {
                app.screen = Screen::TimerRun;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_timer_run_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') => app.screen = Screen::TimerList,
        KeyCode::Char('r') => app.config = ensure_config()?,
        KeyCode::Left => {
            app.digit_font_index = if app.digit_font_index == 0 {
                DIGIT_FONT_NAMES.len() - 1
            } else {
                app.digit_font_index - 1
            };
        }
        KeyCode::Right => {
            app.digit_font_index = (app.digit_font_index + 1) % DIGIT_FONT_NAMES.len();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.config.timers.is_empty() {
                app.timer_index = if app.timer_index == 0 {
                    app.config.timers.len() - 1
                } else {
                    app.timer_index - 1
                };
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.config.timers.is_empty() {
                app.timer_index = (app.timer_index + 1) % app.config.timers.len();
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => app.screen = Screen::Menu,
        KeyCode::Tab | KeyCode::Down => app.add_field = (app.add_field + 1).min(4),
        KeyCode::BackTab | KeyCode::Up => app.add_field = app.add_field.saturating_sub(1),
        KeyCode::Left => {
            if app.add_field == 2 {
                app.add_tz_index = app.add_tz_index.saturating_sub(1);
            }
        }
        KeyCode::Right => {
            if app.add_field == 2 {
                app.add_tz_index =
                    (app.add_tz_index + 1).min(app.config.timezone_presets.len() - 1);
            }
        }
        KeyCode::Enter => {
            if app.add_field == 4 {
                save_new_timer(app)?;
            } else {
                app.add_field = (app.add_field + 1).min(4);
            }
        }
        KeyCode::Backspace => active_add_field_mut(app).pop().map(|_| ()).unwrap_or(()),
        KeyCode::Char(c) => {
            if app.add_field != 2 && app.add_field != 4 {
                active_add_field_mut(app).push(c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn active_add_field_mut(app: &mut App) -> &mut String {
    match app.add_field {
        0 => &mut app.add_title,
        1 => &mut app.add_target,
        3 => &mut app.add_note,
        _ => &mut app.message,
    }
}

fn save_new_timer(app: &mut App) -> io::Result<()> {
    let title = if app.add_title.trim().is_empty() {
        "Untitled Timer".to_string()
    } else {
        app.add_title.trim().to_string()
    };
    let Some(preset) = app.config.timezone_presets.get(app.add_tz_index) else {
        app.message = "无效时区选择".to_string();
        return Ok(());
    };
    let zone_name = preset.zone.clone();
    match parse_local_target(&app.add_target, &zone_name) {
        Ok(target) => {
            app.config.timers.push(TimerConfig {
                id: short_id(),
                title,
                target,
                timezone: zone_name.clone(),
                note: app.add_note.trim().to_string(),
                created_at: Utc::now().to_rfc3339(),
            });
            app.config.settings.default_add_timezone = zone_name;
            save_config(&app.config)?;
            app.add_title.clear();
            app.add_target.clear();
            app.add_note.clear();
            app.message = tr(app.lang(), "added").to_string();
            app.screen = Screen::TimerList;
        }
        Err(err) => app.message = err,
    }
    Ok(())
}

fn handle_settings_key(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            save_config(&app.config)?;
            app.screen = Screen::Menu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings_index = app.settings_index.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => app.settings_index = (app.settings_index + 1).min(3),
        KeyCode::Left => cycle_setting(app, false)?,
        KeyCode::Right | KeyCode::Enter => cycle_setting(app, true)?,
        _ => {}
    }
    Ok(())
}

fn cycle_setting(app: &mut App, forward: bool) -> io::Result<()> {
    match app.settings_index {
        0 => {
            let current = preset_index_app(app, &app.config.settings.home_timezone);
            let next = cycle_index(current, app.config.timezone_presets.len(), forward);
            app.config.settings.home_timezone = app.config.timezone_presets[next].zone.clone();
        }
        1 => {
            let current = PRECISIONS
                .iter()
                .position(|(key, _)| *key == app.config.settings.display_precision)
                .unwrap_or(1);
            let next = cycle_index(current, PRECISIONS.len(), forward);
            app.config.settings.display_precision = PRECISIONS[next].0.to_string();
        }
        2 => {
            let current = preset_index_app(app, &app.config.settings.default_add_timezone);
            let next = cycle_index(current, app.config.timezone_presets.len(), forward);
            app.config.settings.default_add_timezone =
                app.config.timezone_presets[next].zone.clone();
            app.add_tz_index = next;
        }
        3 => {
            let current = LANGUAGES
                .iter()
                .position(|(code, _)| *code == app.config.settings.language)
                .unwrap_or(0);
            let next = cycle_index(current, LANGUAGES.len(), forward);
            app.config.settings.language = LANGUAGES[next].0.to_string();
        }
        _ => {}
    }
    save_config(&app.config)
}

fn cycle_index(current: usize, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if forward {
        (current + 1) % len
    } else if current == 0 {
        len - 1
    } else {
        current - 1
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    match app.screen {
        Screen::Menu => draw_menu(frame, app),
        Screen::TimerList => draw_timer_list(frame, app),
        Screen::TimerRun => draw_timer_run(frame, app),
        Screen::Add => draw_add(frame, app),
        Screen::Settings => draw_settings(frame, app),
        Screen::ConfigView => draw_config_view(frame, app),
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    if area.width < 20 || area.height < 10 {
        return area;
    }
    let w = width
        .min(area.width.saturating_sub(4))
        .max(20)
        .min(area.width);
    let h = height
        .min(area.height.saturating_sub(2))
        .max(10)
        .min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    }
}

fn title_art_lines() -> Vec<Line<'static>> {
    TITLE_ART
        .iter()
        .map(|line| Line::from(*line).alignment(Alignment::Center))
        .collect()
}

fn draw_menu(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let area = centered_rect(frame.area(), 64, 22);
    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL);
    frame.render_widget(block, area);
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 3,
    });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(Paragraph::new(title_art_lines()), rows[0]);
    let summary = format!(
        "{}: {}\n{}: {}\n{}: {}",
        tr(lang, "config"),
        config_path().display(),
        tr(lang, "home_tz"),
        app.config.settings.home_timezone,
        tr(lang, "precision"),
        precision_label_lang(&app.config.settings.display_precision, lang)
    );
    frame.render_widget(
        Paragraph::new(summary).alignment(Alignment::Center),
        rows[1],
    );
    let items = [
        tr(lang, "menu_start"),
        tr(lang, "menu_add"),
        tr(lang, "menu_settings"),
        tr(lang, "menu_config"),
        tr(lang, "menu_quit"),
    ]
    .iter()
    .enumerate()
    .map(|(index, item)| {
        if index == app.menu_index {
            ListItem::new(Line::from(vec![
                Span::raw("› "),
                Span::styled(*item, Style::default().add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(format!("  {item}")))
        }
    })
    .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.menu_index));
    frame.render_stateful_widget(List::new(items).highlight_symbol(""), rows[2], &mut state);
    frame.render_widget(
        Paragraph::new(tr(lang, "menu_help")).alignment(Alignment::Center),
        rows[3],
    );
}

fn draw_timer_list(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let area = centered_rect(frame.area(), 78, 24);
    let block = Block::default()
        .title(format!(" {} ", tr(lang, "timer_list")))
        .borders(Borders::ALL);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    if app.config.timers.is_empty() {
        frame.render_widget(
            Paragraph::new(tr(lang, "no_timers")).alignment(Alignment::Center),
            inner,
        );
        return;
    }
    let now = Utc::now();
    let items = app
        .config
        .timers
        .iter()
        .map(|timer| {
            let remain = target_utc(timer).map(|target| target - now);
            let suffix = remain
                .map(|d| {
                    if d.num_milliseconds() <= 0 {
                        tr(lang, "finished").to_string()
                    } else {
                        format_duration(d, "seconds")
                    }
                })
                .unwrap_or_else(|_| tr(lang, "invalid_time").to_string());
            ListItem::new(format!("{}  {}  [{}]", timer.title, suffix, timer.timezone))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.timer_index.min(app.config.timers.len() - 1)));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("› "),
        inner,
        &mut state,
    );
}

fn draw_timer_run(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let Some(timer) = app.selected_timer() else {
        draw_timer_list(frame, app);
        return;
    };
    let precision = &app.config.settings.display_precision;
    let remain = target_utc(timer)
        .map(|target| target - Utc::now())
        .unwrap_or_else(|_| chrono::Duration::zero());
    let text = if remain.num_milliseconds() <= 0 {
        format_duration(chrono::Duration::zero(), precision)
    } else {
        format_duration(remain, precision)
    };

    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(title_art_lines()), chunks[0]);
    frame.render_widget(
        Paragraph::new(format!("{} · {}", tr(lang, "distance"), timer.title))
            .alignment(Alignment::Center),
        chunks[1],
    );

    render_countdown_text(frame, chunks[2], &text, app.digit_font());

    let mut meta = vec![Line::from(format!(
        "{}: {} [{}]",
        tr(lang, "target"),
        timer.target,
        timer.timezone
    ))];
    if !timer.note.trim().is_empty() {
        meta.push(Line::from(timer.note.clone()));
    }
    frame.render_widget(Paragraph::new(meta).alignment(Alignment::Center), chunks[3]);
    frame.render_widget(
        Paragraph::new(tr(lang, "timer_footer").replace("{font}", app.digit_font_name()))
            .alignment(Alignment::Center),
        chunks[4],
    );
}

#[derive(Clone, Copy)]
enum DigitFont {
    Tube,
    Block,
    Compact,
}

fn render_countdown_text(frame: &mut Frame, area: Rect, text: &str, font: DigitFont) {
    if render_digit_countdown(frame, area, text, font) {
        return;
    }

    if should_use_plain_countdown(area, text) {
        frame.render_widget(
            Paragraph::new(text.to_string())
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::BOLD)),
            area,
        );
        return;
    }

    let pixel_size = countdown_pixel_size(area, text);
    let big_text = BigText::builder()
        .pixel_size(pixel_size)
        .centered()
        .lines(vec![Line::from(text.to_string())])
        .build();
    frame.render_widget(big_text, area);
}

fn render_digit_countdown(frame: &mut Frame, area: Rect, text: &str, font: DigitFont) -> bool {
    if area.height < 8 {
        return false;
    }
    let glyphs = text
        .chars()
        .map(|ch| digit_glyph(font, ch))
        .collect::<Option<Vec<_>>>();
    let Some(glyphs) = glyphs else {
        return false;
    };
    let base_height = glyphs.first().map(|glyph| glyph.len()).unwrap_or(0);
    let base_width = glyphs
        .iter()
        .map(|glyph| glyph.first().map(|line| line.chars().count()).unwrap_or(0))
        .sum::<usize>()
        + glyphs.len().saturating_sub(1);
    if base_height == 0 || base_width == 0 {
        return false;
    }
    let max_x = (area.width.saturating_sub(4) as usize / base_width).max(1);
    let max_y = (area.height.saturating_sub(2) as usize / base_height).max(1);
    let scale = max_x.min(max_y).max(1);
    if scale == 1 && area.width < base_width as u16 {
        return false;
    }
    let base_lines = stitch_glyphs(&glyphs);
    let lines = scale_digit_lines(base_lines, scale);
    let digit_height = lines.len() as u16;
    let target = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(digit_height) / 2,
        width: area.width,
        height: digit_height.min(area.height),
    };
    let rendered = lines
        .into_iter()
        .map(|line| Line::from(line).alignment(Alignment::Center))
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(rendered), target);
    true
}

fn stitch_glyphs(glyphs: &[&[&str]]) -> Vec<String> {
    let height = glyphs.first().map(|glyph| glyph.len()).unwrap_or(0);
    (0..height)
        .map(|row| {
            glyphs
                .iter()
                .map(|glyph| glyph[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn scale_digit_lines(lines: Vec<String>, scale: usize) -> Vec<String> {
    lines
        .into_iter()
        .flat_map(|line| {
            let scaled = line
                .chars()
                .flat_map(|ch| std::iter::repeat(ch).take(scale))
                .collect::<String>();
            std::iter::repeat(scaled).take(scale).collect::<Vec<_>>()
        })
        .collect()
}

fn digit_glyph(font: DigitFont, ch: char) -> Option<&'static [&'static str]> {
    match font {
        DigitFont::Tube => tube_glyph(ch),
        DigitFont::Block => block_glyph(ch),
        DigitFont::Compact => compact_glyph(ch),
    }
}

fn tube_glyph(ch: char) -> Option<&'static [&'static str]> {
    match ch {
        '0' => Some(&[
            " ███ ",
            "█   █",
            "█   █",
            "█   █",
            "█   █",
            "█   █",
            " ███ ",
        ]),
        '1' => Some(&[
            "  █  ",
            " ██  ",
            "  █  ",
            "  █  ",
            "  █  ",
            "  █  ",
            "█████",
        ]),
        '2' => Some(&[
            "████ ",
            "    █",
            "    █",
            " ███ ",
            "█    ",
            "█    ",
            "█████",
        ]),
        '3' => Some(&[
            "████ ",
            "    █",
            "    █",
            " ███ ",
            "    █",
            "    █",
            "████ ",
        ]),
        '4' => Some(&[
            "█   █",
            "█   █",
            "█   █",
            "█████",
            "    █",
            "    █",
            "    █",
        ]),
        '5' => Some(&[
            "█████",
            "█    ",
            "█    ",
            "████ ",
            "    █",
            "    █",
            "████ ",
        ]),
        '6' => Some(&[
            " ███ ",
            "█    ",
            "█    ",
            "████ ",
            "█   █",
            "█   █",
            " ███ ",
        ]),
        '7' => Some(&[
            "█████",
            "    █",
            "   █ ",
            "  █  ",
            " █   ",
            "█    ",
            "█    ",
        ]),
        '8' => Some(&[
            " ███ ",
            "█   █",
            "█   █",
            " ███ ",
            "█   █",
            "█   █",
            " ███ ",
        ]),
        '9' => Some(&[
            " ███ ",
            "█   █",
            "█   █",
            " ████",
            "    █",
            "    █",
            " ███ ",
        ]),
        ':' => Some(&[
            "     ", "  █  ", "  █  ", "     ", "  █  ", "  █  ", "     ",
        ]),
        '.' => Some(&[
            "     ",
            "     ",
            "     ",
            "     ",
            "     ",
            " ██  ",
            " ██  ",
        ]),
        _ => None,
    }
}

fn block_glyph(ch: char) -> Option<&'static [&'static str]> {
    match ch {
        '0' => Some(&[
            "█████",
            "█   █",
            "█  ██",
            "█ █ █",
            "██  █",
            "█   █",
            "█████",
        ]),
        '1' => Some(&[
            "  █  ",
            " ██  ",
            "█ █  ",
            "  █  ",
            "  █  ",
            "  █  ",
            "█████",
        ]),
        '2' => Some(&[
            "█████",
            "    █",
            "    █",
            "█████",
            "█    ",
            "█    ",
            "█████",
        ]),
        '3' => Some(&[
            "█████",
            "    █",
            "    █",
            " ████",
            "    █",
            "    █",
            "█████",
        ]),
        '4' => Some(&[
            "█   █",
            "█   █",
            "█   █",
            "█████",
            "    █",
            "    █",
            "    █",
        ]),
        '5' => Some(&[
            "█████",
            "█    ",
            "█    ",
            "█████",
            "    █",
            "    █",
            "█████",
        ]),
        '6' => Some(&[
            "█████",
            "█    ",
            "█    ",
            "█████",
            "█   █",
            "█   █",
            "█████",
        ]),
        '7' => Some(&[
            "█████",
            "    █",
            "   █ ",
            "  █  ",
            " █   ",
            "█    ",
            "█    ",
        ]),
        '8' => Some(&[
            "█████",
            "█   █",
            "█   █",
            "█████",
            "█   █",
            "█   █",
            "█████",
        ]),
        '9' => Some(&[
            "█████",
            "█   █",
            "█   █",
            "█████",
            "    █",
            "    █",
            "█████",
        ]),
        ':' => Some(&[
            "     ",
            " ██  ",
            " ██  ",
            "     ",
            " ██  ",
            " ██  ",
            "     ",
        ]),
        '.' => Some(&[
            "     ",
            "     ",
            "     ",
            "     ",
            "     ",
            " ██  ",
            " ██  ",
        ]),
        _ => None,
    }
}

fn compact_glyph(ch: char) -> Option<&'static [&'static str]> {
    match ch {
        '0' => Some(&["███", "█ █", "█ █", "█ █", "███"]),
        '1' => Some(&[" █ ", "██ ", " █ ", " █ ", "███"]),
        '2' => Some(&["███", "  █", "███", "█  ", "███"]),
        '3' => Some(&["███", "  █", " ██", "  █", "███"]),
        '4' => Some(&["█ █", "█ █", "███", "  █", "  █"]),
        '5' => Some(&["███", "█  ", "███", "  █", "███"]),
        '6' => Some(&["███", "█  ", "███", "█ █", "███"]),
        '7' => Some(&["███", "  █", " █ ", "█  ", "█  "]),
        '8' => Some(&["███", "█ █", "███", "█ █", "███"]),
        '9' => Some(&["███", "█ █", "███", "  █", "███"]),
        ':' => Some(&["   ", " █ ", "   ", " █ ", "   "]),
        '.' => Some(&["   ", "   ", "   ", "   ", " █ "]),
        _ => None,
    }
}

fn should_use_plain_countdown(area: Rect, text: &str) -> bool {
    area.height < 8 || area.width < text_width_for_pixel_size(text, PixelSize::Quadrant) + 2
}

fn countdown_pixel_size(area: Rect, text: &str) -> PixelSize {
    let candidates = [
        PixelSize::Full,
        PixelSize::HalfHeight,
        PixelSize::HalfWidth,
        PixelSize::Quadrant,
        PixelSize::Sextant,
    ];
    for candidate in candidates {
        if area.width >= text_width_for_pixel_size(text, candidate) + 2
            && area.height >= text_height_for_pixel_size(candidate)
        {
            return candidate;
        }
    }
    PixelSize::Quadrant
}

fn text_width_for_pixel_size(text: &str, pixel_size: PixelSize) -> u16 {
    let chars = text.chars().count() as u16;
    let glyph_width = match pixel_size {
        PixelSize::Full | PixelSize::HalfHeight => 8,
        PixelSize::HalfWidth | PixelSize::Quadrant => 4,
        PixelSize::ThirdHeight => 8,
        PixelSize::Sextant => 3,
        PixelSize::QuarterHeight => 8,
        PixelSize::Octant => 4,
    };
    chars
        .saturating_mul(glyph_width)
        .saturating_add(chars.saturating_sub(1))
}

fn text_height_for_pixel_size(pixel_size: PixelSize) -> u16 {
    match pixel_size {
        PixelSize::Full | PixelSize::HalfWidth => 8,
        PixelSize::HalfHeight | PixelSize::Quadrant => 4,
        PixelSize::ThirdHeight => 3,
        PixelSize::Sextant => 3,
        PixelSize::QuarterHeight | PixelSize::Octant => 2,
    }
}

fn draw_add(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let area = centered_rect(frame.area(), 78, 23);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title(format!(" {} ", tr(lang, "menu_add")))
            .borders(Borders::ALL),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 3,
    });
    let tz = app.config.timezone_presets.get(app.add_tz_index);
    let tz_text = tz
        .map(|item| format!("{} ({})", item.label, item.zone))
        .unwrap_or_else(|| "-".to_string());
    let labels = [
        format!("{}: {}", tr(lang, "add_title"), app.add_title),
        format!("{}: {}", tr(lang, "add_target"), app.add_target),
        format!("{}: {}", tr(lang, "timezone"), tz_text),
        format!("{}: {}", tr(lang, "note"), app.add_note),
        tr(lang, "save").to_string(),
    ];
    let mut lines = vec![
        Line::from(tr(lang, "add_hint_time")),
        Line::from(tr(lang, "add_hint_tz")),
        Line::from(""),
    ];
    for (index, label) in labels.iter().enumerate() {
        let prefix = if index == app.add_field { "› " } else { "  " };
        let style = if index == app.add_field {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(label.clone(), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(app.message.clone()));
    lines.push(Line::from(""));
    lines.push(Line::from(tr(lang, "add_footer")));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_settings(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let area = centered_rect(frame.area(), 76, 21);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title(format!(" {} ", tr(lang, "menu_settings")))
            .borders(Borders::ALL),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 3,
    });
    let rows = [
        format!(
            "{}: {}",
            tr(lang, "home_tz"),
            app.config.settings.home_timezone
        ),
        format!(
            "{}: {}",
            tr(lang, "precision"),
            precision_label_lang(&app.config.settings.display_precision, lang)
        ),
        format!(
            "{}: {}",
            tr(lang, "default_add_tz"),
            app.config.settings.default_add_timezone
        ),
        format!(
            "{}: {}",
            tr(lang, "language"),
            language_label(&app.config.settings.language)
        ),
    ];
    let mut lines = vec![Line::from(tr(lang, "settings_hint")), Line::from("")];
    for (index, row) in rows.iter().enumerate() {
        let style = if index == app.settings_index {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let prefix = if index == app.settings_index {
            "› "
        } else {
            "  "
        };
        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(row.clone(), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(tr(lang, "settings_footer")));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_config_view(frame: &mut Frame, app: &App) {
    let lang = app.lang();
    let area = centered_rect(frame.area(), 96, 28);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title(format!(" {} ", tr(lang, "config_title")))
            .borders(Borders::ALL),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let text = fs::read_to_string(config_path()).unwrap_or_else(|err| format!("读取失败: {err}"));
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn config_path() -> PathBuf {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("toptimer").join("config.json")
}

fn ensure_config() -> io::Result<Config> {
    let path = config_path();
    if !path.exists() {
        save_config(&default_config())?;
    }
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn save_config(config: &Config) -> io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, format!("{raw}\n"))
}

fn default_config() -> Config {
    Config {
        version: 2,
        settings: Settings {
            home_timezone: "Asia/Shanghai".to_string(),
            display_precision: "seconds".to_string(),
            default_add_timezone: "Asia/Shanghai".to_string(),
            language: default_language(),
        },
        timezone_presets: COMMON_TIMEZONES
            .iter()
            .map(|(label, zone)| TimezonePreset {
                label: label.to_string(),
                zone: zone.to_string(),
            })
            .collect(),
        timers: Vec::new(),
    }
}

fn parse_local_target(input: &str, zone_name: &str) -> Result<String, String> {
    let normalized = input.trim().replace('T', " ");
    let naive = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M"))
        .map_err(|_| "时间格式用 YYYY-MM-DD HH:MM".to_string())?;
    let dt = localize(naive, zone_name)?;
    Ok(dt.to_rfc3339())
}

fn localize(naive: NaiveDateTime, zone_name: &str) -> Result<DateTime<FixedOffset>, String> {
    if zone_name == "Etc/GMT+12" {
        return Ok(FixedOffset::west_opt(12 * 3600)
            .unwrap()
            .from_local_datetime(&naive)
            .single()
            .unwrap());
    }
    if zone_name == "UTC" {
        return Ok(FixedOffset::east_opt(0)
            .unwrap()
            .from_local_datetime(&naive)
            .single()
            .unwrap());
    }
    let tz = Tz::from_str(zone_name).map_err(|_| format!("未知时区: {zone_name}"))?;
    let local = match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(a, _) => a,
        LocalResult::None => {
            return Err("这个本地时间在所选时区不存在，通常是夏令时跳变导致".to_string())
        }
    };
    Ok(local.with_timezone(&local.offset().fix()))
}

fn target_utc(timer: &TimerConfig) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(&timer.target).map(|dt| dt.with_timezone(&Utc))
}

fn format_duration(duration: chrono::Duration, precision: &str) -> String {
    let millis = duration.num_milliseconds().max(0);
    let total_seconds = millis / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    match precision {
        "minutes" => format!("{hours:02}:{minutes:02}"),
        "tenths" => format!(
            "{hours:02}:{minutes:02}:{seconds:02}.{}",
            (millis % 1000) / 100
        ),
        "hundredths" => format!(
            "{hours:02}:{minutes:02}:{seconds:02}.{:02}",
            (millis % 1000) / 10
        ),
        "milliseconds" => format!("{hours:02}:{minutes:02}:{seconds:02}.{:03}", millis % 1000),
        _ => format!("{hours:02}:{minutes:02}:{seconds:02}"),
    }
}

fn precision_label_lang(key: &str, lang: Lang) -> &'static str {
    match (lang, key) {
        (Lang::Zh, "minutes") => "分钟 HH:MM",
        (Lang::Zh, "seconds") => "秒 HH:MM:SS",
        (Lang::Zh, "tenths") => "十分之一秒 HH:MM:SS.d",
        (Lang::Zh, "hundredths") => "百分之一秒 HH:MM:SS.dd",
        (Lang::Zh, "milliseconds") => "毫秒 HH:MM:SS.ddd",
        (Lang::Ja, "minutes") => "分 HH:MM",
        (Lang::Ja, "seconds") => "秒 HH:MM:SS",
        (Lang::Ja, "tenths") => "1/10秒 HH:MM:SS.d",
        (Lang::Ja, "hundredths") => "1/100秒 HH:MM:SS.dd",
        (Lang::Ja, "milliseconds") => "ミリ秒 HH:MM:SS.ddd",
        (Lang::En, "minutes") => "minutes HH:MM",
        (Lang::En, "seconds") => "seconds HH:MM:SS",
        (Lang::En, "tenths") => "tenths HH:MM:SS.d",
        (Lang::En, "hundredths") => "hundredths HH:MM:SS.dd",
        (Lang::En, "milliseconds") => "milliseconds HH:MM:SS.ddd",
        _ => "seconds HH:MM:SS",
    }
}

fn preset_index_app(app: &App, zone: &str) -> usize {
    app.config
        .timezone_presets
        .iter()
        .position(|preset| preset.zone == zone)
        .unwrap_or(0)
}

fn preset_index_config(config: &Config, zone: &str) -> usize {
    config
        .timezone_presets
        .iter()
        .position(|preset| preset.zone == zone)
        .unwrap_or(0)
}

fn refresh_interval(config: &Config) -> Duration {
    match config.settings.display_precision.as_str() {
        "milliseconds" => Duration::from_millis(30),
        "hundredths" => Duration::from_millis(40),
        "tenths" => Duration::from_millis(80),
        _ => Duration::from_millis(250),
    }
}

fn short_id() -> String {
    format!("{:x}", Utc::now().timestamp_nanos_opt().unwrap_or_default())
        .chars()
        .rev()
        .take(10)
        .collect()
}
