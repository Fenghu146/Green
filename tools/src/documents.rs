//! The generated Markdown: four documents, built from one seed.

/// "greenote", the only source of variation in the generated text.
pub const SEED: u64 = 0x6772_6565_6e6f_7465;

struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        Rng(SEED)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const SENTENCES: &[&str] = &[
    "折叠只改变视图，文件本身一字不动。",
    "大纲按标题层级排序，不读列表项。",
    "行内公式随文排版，块级公式独立成行。",
    "标签切换后恢复滚动位置与折叠状态。",
    "导出的 PDF 只嵌入用到的字体子集。",
    "导出后的日志记页数、总字节与字体字节。",
    "单页均值超过五百 KB 时给出提示。",
    "主题分深浅两套，跟随系统时读一次系统设置。",
    "正文单栏居中，宽度四十八 rem。",
    "标题字号按层级递减，最多六级。",
    "高亮用两个等号包住文字，文件里原样保留。",
    "查找从当前光标位置开始，循环到文档开头。",
    "保存先写临时文件再原子替换。",
    "外部改动会被发现，冲突时不覆盖磁盘内容。",
    "恢复会话只在正常退出时写入。",
    "图片按相对路径引用，导出时内联。",
    "表格对齐方式由表头分隔行决定。",
    "代码块保留原始空白，不做行内排版。",
    "引用块可以嵌套，最多三层。",
    "列表项里的段落需要缩进对齐。",
    "硬换行以两个空格结尾，软换行由渲染器处理。",
    "字数统计按 Unicode 标量计，与文件字符数一致。",
    "首屏一秒内出现是在普通笔记本上的目标。",
    "滚动掉帧率按每秒六十帧计算。",
    "长任务超过五十毫秒一次即算不达标。",
    "内存占用以单标签两百 MB 为上限。",
];

const EXTRAS: &[&str] = &[
    "设置项写在 `settings.json` 里，键名与界面一致。",
    "这一条要 ==重点标记==，导出时对应 `#highlight[...]`。",
    "**保存失败不弹提示，只在状态栏说明原因。**",
    "参考 [设计文档 §9](https://example.com/docs/design.md) 的阈值。",
    "公式示例：$p_{95} \\le 16\\ \\mathrm{ms}$ 表示尾部延迟。",
];

const SECTION_TITLES: &[&str] = &[
    "折叠与大纲",
    "正文排版",
    "公式与导出",
    "文件与恢复",
    "标签与滚动",
    "主题与字体",
    "查找与替换",
    "插件与外部进程",
    "打包与体积",
    "许可与依赖",
    "性能测量",
    "已知限制",
    "测试与夹具",
    "附录与术语",
];

fn sentence(rng: &mut Rng) -> &'static str {
    let index = rng.below(SENTENCES.len());
    SENTENCES[index]
}

fn paragraph(rng: &mut Rng) -> String {
    let count = 5 + rng.below(4);
    let mut out = String::new();
    for _ in 0..count {
        let index = rng.below(SENTENCES.len());
        out.push_str(SENTENCES[index]);
    }
    if rng.below(6) == 0 {
        let index = rng.below(EXTRAS.len());
        out.push_str(EXTRAS[index]);
    }
    out.push_str("\n\n");
    out
}

fn heading(number: usize) -> String {
    let title = SECTION_TITLES[number % SECTION_TITLES.len()];
    format!("## 第 {number} 节 · {title}\n\n")
}

/// Grows `text` with sentences until it is exactly `target` characters long.
fn fit(mut text: String, target: usize, rng: &mut Rng) -> String {
    let mut length = text.chars().count();
    while length < target {
        let need = target - length;
        let mut tail = String::from(sentence(rng));
        if tail.chars().count() > need {
            tail = tail.chars().take(need.saturating_sub(1)).collect();
            tail.push('。');
        }
        length += tail.chars().count();
        text.push_str(&tail);
    }
    if length > target {
        text = text.chars().take(target - 1).collect();
        text.push('。');
    }
    text
}

pub(crate) fn fifty_thousand_paragraphs() -> String {
    let mut rng = Rng::new();
    let mut out = String::from("# 性能夹具：五万字、三百五十段\n\n");
    for index in 0..350 {
        if index % 25 == 0 {
            out.push_str(&heading(index / 25 + 1));
        }
        out.push_str(&paragraph(&mut rng));
    }
    let mut text = fit(out.trim_end().to_string(), 49_999, &mut rng);
    text.push('\n');
    text
}

pub(crate) fn fifty_thousand_one_line() -> String {
    let mut rng = Rng::new();
    let mut out = String::new();
    while out.chars().count() < 49_500 {
        out.push_str(sentence(&mut rng));
    }
    fit(out, 50_000, &mut rng)
}

pub(crate) fn five_megabytes() -> String {
    let mut rng = Rng::new();
    let mut out = String::from("# 性能夹具：五 MB 大文档\n\n");
    out.push_str("正文约十万字，附录是一张长表与一段原始日志；总量补到五 MB。\n\n");
    for index in 0..820 {
        if index % 40 == 0 {
            out.push_str(&heading(index / 40 + 1));
        }
        out.push_str(&paragraph(&mut rng));
    }
    out.push_str(
        "## 附录：逐行记录\n\n| 序号 | 时间 | 项目 | 备注 |\n| ---: | --- | --- | --- |\n",
    );
    for row in 0..24_000 {
        out.push_str(&format!(
            "| {row} | 2026-11-26T09:{:02}:{:02}Z | {} | note-{:05} |\n",
            rng.below(60),
            rng.below(60),
            SENTENCES[rng.below(SENTENCES.len())].trim_end_matches('。'),
            row,
        ));
    }
    out.push_str("\n## 附录：原始日志\n\n```text\n");
    let tail = "```\n";
    let target = 5_000_000 - tail.len();
    while out.len() + 120 < target {
        out.push_str("2026-11-26T09:00:00Z INFO frame drawn k2p=7.4ms scroll=0.0ms cache=hit\n");
    }
    let rest = target - out.len();
    out.push_str(&"x".repeat(rest.saturating_sub(1)));
    out.push('\n');
    out.push_str(tail);
    out
}

/// Pairs of `$`, block first, then inline: the count the homework fixture needs.
pub fn formulas(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('$') {
        let (delimiter, opened) = if rest[at..].starts_with("$$") {
            ("$$", 2)
        } else {
            ("$", 1)
        };
        let after = &rest[at + opened..];
        let Some(end) = after.find(delimiter) else {
            break;
        };
        found.push(&after[..end]);
        rest = &after[end + opened..];
    }
    found
}

pub(crate) fn homework() -> String {
    let mut out = String::from(
        "# 物理作业：力学与电磁学（第三章）\n\n\
         班级：高二（3）班　　姓名：＿＿＿　　日期：2026-11-20\n\n\
         本次作业共 28 个公式，含中文标点；重点处用 ==高亮== 标出。\n\n",
    );
    out.push_str("## 一、质点运动学\n\n平均速度 ");
    out.push_str(&formula(27));
    out.push_str("，速度 ");
    out.push_str(&formula(0));
    out.push_str("。\n\n1. 求加速度 ");
    out.push_str(&formula(1));
    out.push_str("。\n\n2. 求位置：\n\n");
    out.push_str(&formula(2));
    out.push_str("\n\n3. 代入 t = 2 s 得 ");
    out.push_str(&formula(3));
    out.push_str("。==单位不要漏。==\n\n## 二、牛顿定律\n\n由 ");
    out.push_str(&formula(4));
    out.push_str(" 可知，合力为零时加速度为零；水平面上的滑动摩擦力为 ");
    out.push_str(&formula(5));
    out.push_str("。\n\n合力的功写作\n\n");
    out.push_str(&formula(6));
    out.push_str("\n\n## 三、功与能\n\n动能为 ");
    out.push_str(&formula(7));
    out.push_str("，也可写成\n\n");
    out.push_str(&formula(8));
    out.push_str("\n\n其中动量 ");
    out.push_str(&formula(9));
    out.push_str("，动能定理写作\n\n");
    out.push_str(&formula(10));
    out.push_str("\n\n重力加速度取 ");
    out.push_str(&formula(11));
    out.push_str("。\n\n## 四、振动与波\n\n单摆周期为\n\n");
    out.push_str(&formula(12));
    out.push_str("\n\n已知摆长 ");
    out.push_str(&formula(13));
    out.push_str("，角频率 ");
    out.push_str(&formula(14));
    out.push_str("，波速 ");
    out.push_str(&formula(15));
    out.push_str("。\n\n## 五、静电场\n\n点电荷 ");
    out.push_str(&formula(16));
    out.push_str("，库仑定律为\n\n");
    out.push_str(&formula(17));
    out.push_str("\n\n真空介电常数 ");
    out.push_str(&formula(18));
    out.push_str("；场强与匀强电场中的电势差为 ");
    out.push_str(&formula(19));
    out.push_str("。\n\n电容器电容为\n\n");
    out.push_str(&formula(20));
    out.push_str("\n\n电荷量 ");
    out.push_str(&formula(21));
    out.push_str("。\n\n## 六、直流电路\n\n电阻定律 ");
    out.push_str(&formula(22));
    out.push_str("，欧姆定律 ");
    out.push_str(&formula(23));
    out.push_str("，功率为\n\n");
    out.push_str(&formula(24));
    out.push_str("\n\n效率 ");
    out.push_str(&formula(25));
    out.push_str("。\n\n## 七、磁场\n\n安培环路定理为\n\n");
    out.push_str(&formula(26));
    out.push_str("\n\n| 量 | 符号 | 单位 |\n| --- | --- | --- |\n");
    out.push_str("| 速度 | `v` | m/s |\n| 加速度 | `a` | m/s² |\n| 电荷量 | `q` | C |\n\n");
    out.push_str("### 订正\n\n第二题漏了初速度的符号，==已重算==；其余各题按步骤检查通过。\n");
    out
}
pub(crate) const FORMULAS: &[(&str, bool)] = &[
    ("v(t) = 3t^{2} - 4t", false),
    (r"a(t) = \dfrac{\mathrm{d}v}{\mathrm{d}t} = 6t - 4", false),
    (r"x(t) = x_0 + \int_0^t v(\tau)\,\mathrm{d}\tau", true),
    (r"x(2) = 4\ \mathrm{m}", false),
    (r"a = \dfrac{F}{m}", false),
    (r"f_k = \mu_k N", false),
    (r"W = \int_A^B \vec{F} \cdot \mathrm{d}\vec{s}", true),
    (r"E_k = \tfrac{1}{2} m v^{2}", false),
    (r"E_k = \dfrac{p^{2}}{2m}", true),
    ("p = m v", false),
    (r"\Delta E_k = W_{\text{合}}", true),
    (r"g = 9.8\ \mathrm{m\,s^{-2}}", false),
    (r"T = 2\pi\sqrt{\dfrac{l}{g}}", true),
    (r"l = 0.80\ \mathrm{m}", false),
    (r"\omega = \dfrac{2\pi}{T}", false),
    (r"v = \lambda f", false),
    (r"q = 2.0 \times 10^{-6}\ \mathrm{C}", false),
    (
        r"F = \dfrac{1}{4\pi\varepsilon_0} \dfrac{q_1 q_2}{r^{2}}",
        true,
    ),
    (
        r"\varepsilon_0 = 8.85 \times 10^{-12}\ \mathrm{F\,m^{-1}}",
        false,
    ),
    (r"E = \dfrac{F}{q}, \quad U = E d", false),
    (r"C = \dfrac{\varepsilon_r \varepsilon_0 S}{d}", true),
    ("Q = C U", false),
    (r"R = \rho \dfrac{l}{S}", false),
    (r"I = \dfrac{U}{R}", false),
    (r"P = U I = I^{2} R", true),
    (
        r"\eta = \dfrac{P_{\text{出}}}{P_{\text{入}}} \times 100\%",
        false,
    ),
    (r"\oint \vec{B} \cdot \mathrm{d}\vec{l} = \mu_0 I", true),
    (r"\bar{v} = \dfrac{\Delta s}{\Delta t}", false),
];

fn formula(index: usize) -> String {
    let (tex, block) = FORMULAS[index];
    if block {
        format!("$$ {tex} $$")
    } else {
        format!("${tex}$")
    }
}
