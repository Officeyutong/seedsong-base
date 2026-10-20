mod quality;
use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use seedsong_core::{
    backend::{
        MusicBackend,
        native::{NativeBackend, export},
    },
    dna::Dna,
    music::{KEYS, LEADS, MODES, Params, Song},
    synth::Options,
};
use std::{
    fs::File,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Timbre {
    Spec,
    Piano,
}
impl From<Timbre> for seedsong_core::synth::Timbre {
    fn from(value: Timbre) -> Self {
        match value {
            Timbre::Spec => Self::Spec,
            Timbre::Piano => Self::Piano,
        }
    }
}

#[derive(Parser)]
#[command(
    version,
    about = "Seedsong：32 字节 DNA → 确定性音乐 → 音频文件",
    long_about = "无参数运行可逐步输入。相同 DNA 得到相同 SNG1 音符事件。默认生成立体声 WAV；opus/flac/mp3/m4a 等格式使用 ffmpeg 转换。"
)]
struct Args {
    /// 64 个十六进制字符的 DNA，可带 0x
    #[arg(long, conflicts_with = "random")]
    dna: Option<String>,
    /// 随机生成全部 DNA，并打印以便复现
    #[arg(long)]
    random: bool,
    /// 输出路径，支持 wav/opus/ogg/flac/mp3/m4a/aac
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// 导出规范的 SNG1 二进制音符事件
    #[arg(long)]
    events: Option<PathBuf>,
    /// 仅输出音符事件，不合成音频
    #[arg(long)]
    events_only: bool,
    /// spec 保留 DNA 音色，piano 将旋律渲染为钢琴
    #[arg(long, value_enum, default_value = "spec")]
    timbre: Timbre,
    /// 采样率，可选 22050/44100/48000/96000
    #[arg(long,default_value_t=44100,value_parser=clap::value_parser!(u32).range(22050..=96000))]
    sample_rate: u32,
    /// Rayon 工作线程数，默认使用可用 CPU
    #[arg(long,value_parser=clap::value_parser!(u16).range(1..=256))]
    threads: Option<u16>,
    /// 混响量，0 为关闭，范围 0–1
    #[arg(long, default_value_t = 0.18)]
    reverb: f64,
    /// 将尾音折回开头，输出精确 16 小节的循环 WAV
    #[arg(long = "loop")]
    looping: bool,
    /// 合奏参与者，例如 --part melody=DNA，可重复使用；指挥的声部不可覆盖
    #[arg(long = "part", value_name = "ROLE=DNA")]
    parts: Vec<String>,
    /// 对 N 个固定随机 DNA 并行检查质量，生成 JSON 报告，不合成音频
    #[arg(long,value_parser=clap::value_parser!(u32).range(1..=100000))]
    check: Option<u32>,
    /// 质量报告路径，默认 quality-report.json
    #[arg(long, requires = "check")]
    report: Option<PathBuf>,
}
fn line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut s = String::new();
    ensure!(
        io::stdin().read_line(&mut s)? > 0,
        "输入已结束；可使用 --dna 或 --random 非交互运行"
    );
    Ok(s.trim().to_owned())
}
fn random_answer(s: &str) -> bool {
    s.is_empty() || s.eq_ignore_ascii_case("r") || s == "随机"
}
fn choose(label: &str, options: &[&str]) -> Result<Option<usize>> {
    println!("\n{label}");
    for (i, s) in options.iter().enumerate() {
        println!("  {}. {s}", i + 1);
    }
    loop {
        let s = line("选择编号，或回车 / r 随机：")?;
        if random_answer(&s) {
            return Ok(None);
        }
        if let Ok(n) = s.parse::<usize>()
            && (1..=options.len()).contains(&n)
        {
            return Ok(Some(n - 1));
        }
        println!("请输入 1–{} 或 r。", options.len());
    }
}
fn bytes_prompt(dna: &mut Dna, range: std::ops::Range<usize>, label: &str) -> Result<()> {
    loop {
        let s = line(&format!(
            "{label}（{} 字节十六进制，回车 / r 随机）：",
            range.len()
        ))?;
        if random_answer(&s) {
            return Ok(());
        }
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        if s.len() == range.len() * 2 && s.is_ascii() {
            let b: std::result::Result<Vec<u8>, _> = (0..range.len())
                .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16))
                .collect();
            if let Ok(b) = b {
                dna.0[range.clone()].copy_from_slice(&b);
                return Ok(());
            }
        }
        println!("长度或十六进制字符不正确，请重新输入。");
    }
}
fn wizard() -> Result<Dna> {
    println!("Seedsong · 32 字节 DNA 音乐生成器");
    match choose(
        "DNA 输入方式",
        &["粘贴完整 DNA", "逐步选择音乐参数", "全部随机"],
    )? {
        Some(0) => loop {
            let s = line("请输入完整 DNA（64 个十六进制字符，r 随机）：")?;
            if random_answer(&s) {
                return Ok(Dna::random());
            }
            match Dna::parse(&s) {
                Ok(d) => return Ok(d),
                Err(e) => println!("{e}"),
            }
        },
        Some(1) => {}
        _ => return Ok(Dna::random()),
    }
    let mut dna = Dna::random();
    if let Some(i) = choose("调式", &MODES)? {
        dna.0[0] = [0, 8, 16, 20, 24, 28, 31][i];
    }
    loop {
        let s = line("\n速度 BPM（60–140，回车 / r 随机）：")?;
        if random_answer(&s) {
            break;
        }
        if let Ok(n) = s.parse::<u8>()
            && (60..=140).contains(&n)
        {
            dna.0[1] = n - 60;
            break;
        }
        println!("请输入 60–140 或 r。");
    }
    if let Some(i) = choose(
        "和弦进行模板（按当前调式采用大调族或小调族）",
        &[
            "I–V–vi–IV / i–VI–III–VII",
            "I–vi–IV–V / i–iv–VII–III",
            "vi–IV–I–V / i–VII–VI–VII",
            "I–IV–vi–V / i–iv–v–i",
            "I–V–IV–V / VI–VII–i–i",
            "I–iii–IV–V / i–III–VII–iv",
            "IV–I–V–vi / i–VI–iv–v",
            "I–IV–I–V / i–iv–i–v",
        ],
    )? {
        dna.0[2] = i as u8;
    }
    if let Some(i) = choose("旋律轮廓", &["Smooth 平滑", "Balanced 均衡", "Leapy 跳跃"])?
    {
        dna.0[3] = [0, 3, 7][i];
    }
    if let Some(i) = choose("DNA 主音色（--timbre piano 可另外启用钢琴渲染）", &LEADS)?
    {
        dna.0[4] = [0, 10, 17, 23, 28, 31][i];
    }
    if let Some(i) = choose("低音音色", &["Sine Sub", "Round FM", "Pluck Bass"])? {
        dna.0[5] = [0, 2, 3][i];
    }
    if let Some(i) = choose("突出声部（提高 3 dB）", &["旋律", "低音", "和声", "节奏"])?
    {
        dna.0[16] = i as u8;
    }
    if let Some(i) = choose("主音", &KEYS)? {
        dna.0[17] = i as u8;
    }
    if let Some(i) = choose("曲式", &["AABA", "ABAB", "AABB", "ABAC"])? {
        dna.0[18] = (dna.0[18] & !3) | i as u8;
    }
    if let Some(i) = choose(
        "低音型",
        &["整小节根音", "半小节根音", "根音与五度", "根音与高八度"],
    )? {
        dna.0[18] = (dna.0[18] & !12) | ((i as u8) << 2);
    }
    if let Some(i) = choose("和声垫型", &["长音", "二分音符", "分解和弦", "反拍"])?
    {
        dna.0[18] = (dna.0[18] & !48) | ((i as u8) << 4);
    }
    if let Some(i) = choose("鼓型", &["基本", "半速", "四拍底鼓", "氛围"])? {
        dna.0[18] = (dna.0[18] & !192) | ((i as u8) << 6);
    }
    if let Some(i) = choose("摇摆", &["直拍", "轻摇摆 12%", "摇摆 25%"])? {
        dna.0[19] = (dna.0[19] & !3) | [0, 2, 3][i];
    }
    if let Some(i) = choose("密度偏移", &["正常", "稀疏", "密集"])? {
        dna.0[19] = (dna.0[19] & !12) | ((i as u8) << 2);
    }
    bytes_prompt(&mut dna, 6..10, "旋律种子")?;
    bytes_prompt(&mut dna, 10..16, "封面字节（不影响音乐）")?;
    bytes_prompt(&mut dna, 20..32, "变奏与噪声种子")?;
    Ok(dna)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("无法新建 {}（不会覆盖现有文件）", path.display()))?;
    f.write_all(bytes)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        [22050, 44100, 48000, 96000].contains(&args.sample_rate),
        "采样率须为 22050/44100/48000/96000"
    );
    ensure!(
        args.reverb.is_finite() && (0.0..=1.0).contains(&args.reverb),
        "reverb 必须为 0–1 的有限数值"
    );
    let mut pool = rayon::ThreadPoolBuilder::new();
    if let Some(n) = args.threads {
        pool = pool.num_threads(n as usize);
    }
    let pool = pool.build()?;
    if let Some(count) = args.check {
        let report = pool.install(|| quality::report(count as usize));
        let path = args.report.unwrap_or_else(|| "quality-report.json".into());
        write_new(&path, &serde_json::to_vec_pretty(&report)?)?;
        println!(
            "{}\n报告：{}",
            serde_json::to_string_pretty(&report["summary"])?,
            path.display()
        );
        return Ok(());
    }
    let interactive = args.dna.is_none() && !args.random;
    ensure!(
        !interactive || io::stdin().is_terminal(),
        "非交互环境请提供 --dna 或 --random"
    );
    let dna = if let Some(ref s) = args.dna {
        Dna::parse(s)?
    } else if args.random {
        Dna::random()
    } else {
        wizard()?
    };
    let mut participants = [None; 4];
    let role = Params::new(&dna).role;
    for entry in &args.parts {
        let (part, s) = entry
            .split_once('=')
            .context("--part 格式为 melody=DNA / bass=DNA / pad=DNA / drums=DNA")?;
        let i = match part {
            "melody" => 0,
            "bass" => 1,
            "pad" => 2,
            "drums" => 3,
            _ => bail!("未知声部：{part}"),
        };
        ensure!(i != role as usize, "指挥自己的声部由指挥担任，不能覆盖");
        ensure!(participants[i].is_none(), "重复的声部：{part}");
        let participant = Dna::parse(s)?;
        ensure!(
            Params::new(&participant).role as usize == i,
            "参与 DNA 的 D[16] 必须对应声部 {part}"
        );
        participants[i] = Some(participant);
    }
    let song = pool.install(|| Song::generate(dna, participants));
    let p = &song.params;
    println!(
        "DNA: {dna}\n{} {} · {} BPM · {} · {} 个事件",
        KEYS[p.pc as usize],
        MODES[p.mode as usize],
        p.bpm,
        ["AABA", "ABAB", "AABB", "ABAC"][p.form as usize],
        song.events.len()
    );
    let mut timbre = args.timbre;
    if interactive && !args.events_only {
        timbre = match choose("渲染音色", &["使用 DNA 指定音色", "钢琴旋律（泛音合成）"])?
        {
            Some(0) => Timbre::Spec,
            Some(_) => Timbre::Piano,
            None => {
                if rand::random() {
                    Timbre::Piano
                } else {
                    Timbre::Spec
                }
            }
        };
    }
    let mut output = args
        .output
        .clone()
        .unwrap_or_else(|| format!("seedsong-{}.wav", &dna.to_string()[..12]).into());
    if interactive && args.output.is_none() && !args.events_only {
        let s = line(&format!(
            "\n输出路径（回车使用 {}；扩展名决定格式）：",
            output.display()
        ))?;
        if !s.is_empty() {
            output = s.into();
        }
    }
    let event_path = args
        .events
        .clone()
        .or_else(|| args.events_only.then(|| output.with_extension("sng")));
    if let Some(ref path) = event_path {
        ensure!(!path.exists(), "文件已存在：{}", path.display());
    }
    if !args.events_only {
        let ext = output
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        ensure!(
            ["wav", "flac", "opus", "ogg", "mp3", "m4a", "aac"].contains(&ext.as_str()),
            "不支持的音频扩展名：{ext}"
        );
        ensure!(!output.exists(), "文件已存在：{}", output.display());
        let wav = if ext == "wav" {
            output.clone()
        } else {
            output.with_extension("wav")
        };
        ensure!(!wav.exists(), "中间 WAV 已存在：{}", wav.display());
        if let Some(ref path) = event_path {
            ensure!(
                path != &wav && path != &output,
                "事件文件和音频文件路径必须不同"
            );
        }
        let dna_path = output.with_extension("dna.txt");
        ensure!(
            !dna_path.exists(),
            "DNA 记录文件已存在：{}",
            dna_path.display()
        );
        if let Some(ref path) = event_path {
            ensure!(path != &dna_path, "事件文件与 DNA 记录路径必须不同");
        }
        // Store all participant DNA and render settings alongside every audio export.
        let meta = output.with_extension("json");
        ensure!(
            event_path.as_ref() != Some(&meta),
            "事件文件不能与渲染记录文件同名"
        );
        ensure!(!meta.exists(), "渲染记录已存在：{}", meta.display());
        write_new(&dna_path, format!("{dna}\n").as_bytes())?;
        let metadata = serde_json::json!({"version":"seedsong-music/1","dna":dna.to_string(),"sources":song.sources.map(|d|d.to_string()),
            "timbre":format!("{timbre:?}"),"sample_rate":args.sample_rate,"reverb":args.reverb,"loop":args.looping});
        write_new(&meta, &serde_json::to_vec_pretty(&metadata)?)?;
        let start = Instant::now();
        println!(
            "使用 {} 个线程合成 {:?} 音色…",
            pool.current_num_threads(),
            timbre
        );
        let audio = pool.install(|| {
            NativeBackend.render(
                &song,
                Options {
                    rate: args.sample_rate,
                    timbre: timbre.into(),
                    looping: args.looping,
                    reverb: args.reverb,
                },
            )
        })?;
        pool.install(|| audio.write_wav(&wav))?;
        println!(
            "WAV：{}（{:.2} 秒，耗时 {:.2} 秒）",
            wav.display(),
            audio.frames.len() as f64 / args.sample_rate as f64,
            start.elapsed().as_secs_f64()
        );
        if ext != "wav" {
            export(&wav, &output)?;
            println!("音频：{}", output.display());
        }
        println!("复现记录：{}", meta.display());
    }
    if let Some(path) = event_path {
        write_new(&path, &song.encode())?;
        println!("SNG1：{}", path.display());
    }
    Ok(())
}
