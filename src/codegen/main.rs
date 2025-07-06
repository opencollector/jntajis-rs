use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::BufReader;

use lazy_static::lazy_static;
use serde::Deserialize;

use calamine::{DataType as _, Reader as _, Xlsx, open_workbook};

use jntajis::codec::array_vec::ArrayVec;
use jntajis::codec::common_models::{
    Ivs, JISCharacterClass, JNTAMapping, MJCode, MenKuTen, UIVSPair,
};
use jntajis::codec::inwire_models;

#[derive(Clone, Default, Debug)]
struct MJShrinkMappingUnicodeSet {
    jis_incorporation_ucs_unification_rule: Vec<u32>,
    inference_by_reading_and_glyph: Vec<u32>,
    moj_notice_582: Vec<u32>,
    moj_family_register_act_related_notice: Vec<u32>,
}

impl MJShrinkMappingUnicodeSet {
    fn is_empty(&self) -> bool {
        self.jis_incorporation_ucs_unification_rule.is_empty()
            && self.inference_by_reading_and_glyph.is_empty()
            && self.moj_notice_582.is_empty()
            && self.moj_family_register_act_related_notice.is_empty()
    }
}

struct MJShrinkMapping {
    mj: MJCode,
    us: MJShrinkMappingUnicodeSet,
}

lazy_static! {
    static ref JIS_CHARACTER_CLASS: BTreeMap<&'static str, JISCharacterClass> = {
        let mut m = BTreeMap::new();
        m.insert("非漢字", JISCharacterClass::JISX0208NonKanji);
        m.insert("追加非漢字", JISCharacterClass::JISX0213NonKanji);
        m.insert("JIS1水", JISCharacterClass::KanjiLevel1);
        m.insert("JIS2水", JISCharacterClass::KanjiLevel2);
        m.insert("JIS3水", JISCharacterClass::KanjiLevel3);
        m.insert("JIS4水", JISCharacterClass::KanjiLevel4);
        m
    };
}

lazy_static! {
    static ref REGEXP_UNI_REPR: regex::Regex =
        regex::Regex::new(r"^[uU]\+([0-9A-Fa-f]{4,6})$").expect("should never happen");
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UnicodeReprParseError(String);

impl std::fmt::Display for UnicodeReprParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for UnicodeReprParseError {}

impl From<String> for UnicodeReprParseError {
    fn from(e: String) -> Self {
        Self(e)
    }
}

fn parse_uni_repr(v: impl AsRef<str>) -> Result<u32, UnicodeReprParseError> {
    let v = v.as_ref();
    match REGEXP_UNI_REPR.captures(v) {
        Some(c) => {
            let hex = c.get(1).unwrap().as_str();
            u32::from_str_radix(hex, 16)
                .map_err(|_| UnicodeReprParseError(format!("invalid unicode repr: {}", v)))
        }
        None => Err(UnicodeReprParseError(format!(
            "invalid unicode representation: {}",
            v
        ))),
    }
}

fn parse_uni_seq_repr(v: impl AsRef<str>) -> Result<Vec<u32>, UnicodeReprParseError> {
    v.as_ref()
        .split_whitespace()
        .map(parse_uni_repr)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| UnicodeReprParseError(format!("failed to parse unicode sequence: {}", e)))
}

fn parse_underscore_delimited_uni_hex_repr(
    v: impl AsRef<str>,
) -> Result<Vec<u32>, UnicodeReprParseError> {
    v.as_ref()
        .split('_')
        .map(|s| {
            u32::from_str_radix(s, 16)
                .map_err(|e| UnicodeReprParseError(format!("invalid unicode hex repr: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| UnicodeReprParseError(format!("failed to parse unicode hex sequence: {}", e)))
}

lazy_static! {
    static ref REGEXP_MEMO: regex::Regex =
        regex::Regex::new(r"U\+([0-9A-Fa-f]{4,6})").expect("should never happen");
}

fn read_jnta_excel_file(f: impl AsRef<std::path::Path>) -> Result<Vec<JNTAMapping>, anyhow::Error> {
    let mut book: Xlsx<_> =
        open_workbook(f).map_err(|e| anyhow::Error::from(e).context("failed to open workbook"))?;
    let wss = book.worksheets();
    if wss.is_empty() {
        return Err(anyhow::anyhow!("no worksheets found in the workbook"));
    }
    let ws = &wss[0].1;
    let mut r = ws.rows();
    let mut mappings = Vec::new();

    let row = r
        .next()
        .ok_or(anyhow::anyhow!("too few rows in the worksheet"))?;
    // assert if it is formatted in the expected manner
    if row[0].get_string() != Some("変換元の文字（JISX0213：1-4水）")
        || row[4].get_string() != Some("コード変換（1対1変換）")
        || row[7].get_string() != Some("文字列変換（追加非漢字や、1対ｎの文字変換を行う）")
        || row[16].get_string() != Some("備考")
    {
        return Err(anyhow::anyhow!(
            "a column of the first row does not match to the expected values"
        ));
    }

    let row = r
        .next()
        .ok_or(anyhow::anyhow!("too few rows in the worksheet"))?;
    if row[0].get_string() != Some("面区点コード")
        || row[1].get_string() != Some("Unicode")
        || row[2].get_string() != Some("字形")
        || row[3].get_string() != Some("JIS区分")
        || row[4].get_string() != Some("面区点コード")
        || row[5].get_string() != Some("Unicode")
        || row[6].get_string() != Some("字形")
        || row[7].get_string() != Some("面区点コード①")
        || row[8].get_string() != Some("面区点コード②")
        || row[9].get_string() != Some("面区点コード③")
        || row[10].get_string() != Some("面区点コード④")
        || row[11].get_string() != Some("Unicode①")
        || row[12].get_string() != Some("Unicode②")
        || row[13].get_string() != Some("Unicode③")
        || row[14].get_string() != Some("Unicode④")
        || row[15].get_string() != Some("字形")
    {
        return Err(anyhow::anyhow!(
            "a column of the second row does not match to the expected values"
        ));
    }

    let mut lj: Option<MenKuTen> = None;
    for (ro, row) in r.enumerate() {
        // If the first cell is empty, break
        if row[0].get_string().is_none() || row[0].get_string().unwrap().is_empty() {
            break;
        }

        // Convert Option<&DataType> to String for all columns
        let row: Vec<&str> = row.iter().map(|c| c.get_string().unwrap_or("")).collect();

        // Get class
        let class = match JIS_CHARACTER_CLASS.get(row[3]) {
            Some(&c) => c,
            None => return Err(anyhow::anyhow!("unknown category name: {}", row[3])),
        };

        // Parse jis
        let jis = MenKuTen::from_repr(&row[0]).map_err(|e| {
            anyhow::Error::from(e).context(format!(
                "failed to parse men-ku-ten at row {}: \"{}\"",
                ro + 2,
                row[0]
            ))
        })?;

        // Parse us
        let us = parse_uni_seq_repr(&row[1]).map_err(|e| {
            anyhow::Error::from(e).context(format!(
                "failed to parse rune at row {}: \"{}\"",
                ro + 2,
                row[1]
            ))
        })?;

        if us.len() > 2 {
            return Err(anyhow::anyhow!("too many unicode codepoints: {:?}", us));
        }

        let mut sus: Vec<u32> = Vec::new();
        let mut tx_jis: Vec<MenKuTen> = Vec::new();
        let mut tx_us: Vec<u32> = Vec::new();

        // Fill reserved mappings if there are gaps
        if let Some(lj) = lj {
            if lj + 1 < jis {
                for i in (lj + 1).0..jis.0 {
                    mappings.push(JNTAMapping {
                        jis: MenKuTen(i),
                        us: us.as_slice().try_into()?,
                        sus: ArrayVec::new(),
                        class: JISCharacterClass::Reserved,
                        tx_jis: ArrayVec::new(),
                        tx_us: ArrayVec::new(),
                    });
                }
            }
        }

        // Single mapping
        if !row[4].is_empty() {
            if row[5].is_empty() {
                return Err(anyhow::anyhow!(
                    "non-empty men-ku-ten code followed by empty Unicode at row {}",
                    ro + 2
                ));
            }
            tx_jis = vec![MenKuTen::from_repr(&row[4]).map_err(|e| {
                anyhow::Error::from(e)
                    .context(format!("failed to parse men-ku-ten at row {}", ro + 2))
            })?];
            tx_us = vec![parse_uni_repr(&row[5]).map_err(|e| {
                anyhow::Error::from(e).context(format!("failed to parse rune at row {}", ro + 2))
            })?];
        }
        // Multi mapping
        else if !row[7].is_empty() {
            if row[11].is_empty() {
                return Err(anyhow::anyhow!(
                    "empty single-mapping rune followed by empty runes at row {}",
                    ro + 2
                ));
            }
            // Parse up to 4 men-ku-ten codes
            tx_jis = (7..=10)
                .map(|i| row[i])
                .take_while(|v| !v.is_empty())
                .map(|v| MenKuTen::from_repr(v))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    anyhow::Error::from(e)
                        .context(format!("failed to parse men-ku-ten at row {}", ro + 2))
                })?;
            // Parse up to 4 unicode codepoints
            tx_us = (11..=14)
                .map(|i| row[i])
                .take_while(|v| !v.is_empty())
                .map(parse_uni_repr)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    anyhow::Error::from(e)
                        .context(format!("failed to parse rune at row {}", ro + 2))
                })?;
            if tx_jis.len() != tx_us.len() {
                return Err(anyhow::anyhow!(
                    "number of characters for the transliteration form does not agree between JIS and Unicode at row {}",
                    ro + 2
                ));
            }
        }
        // Parse sus from memo if present
        if !row[16].is_empty() {
            // Try to extract U+XXXX from memo
            if let Some(m) = REGEXP_MEMO.captures(&row[16]) {
                let s = format!("U+{}", &m[1]);
                sus = vec![parse_uni_repr(&s).map_err(|e| {
                    anyhow::Error::from(e).context(format!(
                        "failed to parse rune in memo ({}) at row {}",
                        row[16],
                        ro + 2
                    ))
                })?];
            }
        }
        lj = Some(jis);
        mappings.push(JNTAMapping {
            jis,
            us: us.try_into()?,
            sus: sus.try_into()?,
            class,
            tx_jis: tx_jis.try_into()?,
            tx_us: tx_us.try_into()?,
        });
    }
    // return mappings
    Ok(mappings)
}

static MJ_FIELDS: &[&str] = &[
    "図形",
    "font",
    "MJ文字図形名",
    "対応するUCS",
    "実装したUCS",
    "実装したMoji_JohoコレクションIVS",
    "実装したSVS",
    "戸籍統一文字番号",
    "住基ネット統一文字コード",
    "入管正字コード",
    "入管外字コード",
    "漢字施策",
    "対応する互換漢字",
    "X0213",
    "X0213 包摂連番",
    "X0213 包摂区分",
    "X0212",
    "MJ文字図形バージョン",
    "登記統一文字番号(参考)",
    "部首1(参考)",
    "内画数1(参考)",
    "部首2(参考)",
    "内画数2(参考)",
    "部首3(参考)",
    "内画数3(参考)",
    "部首4(参考)",
    "内画数4(参考)",
    "総画数(参考)",
    "読み(参考)",
    "大漢和",
    "日本語漢字辞典",
    "新大字典",
    "大字源",
    "大漢語林",
    "更新履歴",
    "備考",
];

struct MJMapping {
    mj: MJCode,
    v: BTreeSet<UIVSPair>,
}

fn read_mj_excel_file(f: impl AsRef<std::path::Path>) -> Result<Vec<MJMapping>, anyhow::Error> {
    let mut book: Xlsx<_> =
        open_workbook(f).map_err(|e| anyhow::Error::from(e).context("failed to open workbook"))?;
    let wss = book.worksheets();
    if wss.len() < 1 {
        return Err(anyhow::anyhow!("too few worksheets in the workbook"));
    }
    let ws = &wss[0].1;
    let mut r = ws.rows();

    // Read header row
    let row = r
        .next()
        .ok_or(anyhow::anyhow!("too few rows in the worksheet"))?;
    let header: Vec<&str> = row.iter().map(|c| c.get_string().unwrap_or("")).collect();
    if header.len() < MJ_FIELDS.len() || !header.iter().zip(MJ_FIELDS.iter()).all(|(c, f)| c == f) {
        return Err(anyhow::anyhow!(
            "a column of the first row does not match to the expected values"
        ));
    }

    let mut mappings = Vec::new();

    for row in r {
        let row: Vec<&str> = row.iter().map(|c| c.get_string().unwrap_or("")).collect();
        if row.len() < 7 {
            continue;
        }
        if row[2].is_empty() {
            continue;
        }
        let mj = match MJCode::from_repr(row[2]) {
            Ok(mj) => mj,
            Err(_) => continue,
        };
        if row[3].is_empty() {
            continue;
        }
        let mut uivps = BTreeSet::new();

        // Parse main UCS
        let u = match parse_uni_repr(row[3]) {
            Ok(u) => u,
            Err(_) => continue,
        };
        uivps.insert(UIVSPair { u, s: None });

        // Parse implemented UCS
        if !row[4].is_empty() {
            if let Ok(iu) = parse_uni_repr(row[4]) {
                uivps.insert(UIVSPair { u: iu, s: None });
            }
        }

        // Parse implemented Moji_Joho IVS
        if !row[5].is_empty() {
            for r in row[5].split(';') {
                if r.trim().is_empty() {
                    continue;
                }
                let us = parse_underscore_delimited_uni_hex_repr(r)?;
                if us.len() != 2 {
                    return Err(anyhow::anyhow!("invalid IVS representation: {}", r));
                }
                uivps.insert(UIVSPair {
                    u: us[0],
                    s: Some(Ivs::try_from(us[1])?),
                });
            }
        }

        // Parse implemented SVS
        if !row[6].is_empty() {
            for r in row[6].split(';') {
                if r.trim().is_empty() {
                    continue;
                }
                let us = parse_underscore_delimited_uni_hex_repr(r)?;
                if us.len() != 2 {
                    return Err(anyhow::anyhow!("invalid IVS representation: {}", r));
                }
                uivps.insert(UIVSPair {
                    u: us[0],
                    s: Some(Ivs::try_from(us[1])?),
                });
            }
        }

        mappings.push(MJMapping { mj, v: uivps });
    }

    Ok(mappings)
}

fn read_mj_shrink_file(
    src: impl AsRef<std::path::Path>,
) -> Result<Vec<MJShrinkMapping>, anyhow::Error> {
    #[derive(Deserialize)]
    struct MJFileCodeSet {
        #[serde(rename = "UCS")]
        ucs: String,
    }

    fn deserialize_mj_code<'de, D>(deserializer: D) -> Result<MJCode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        MJCode::from_repr(s).map_err(serde::de::Error::custom)
    }

    #[derive(Deserialize)]
    struct MJFileShrinkMapping {
        #[serde(rename = "JIS包摂規準・UCS統合規則")]
        jis_incorporation_ucs_unification_rule: Option<Vec<MJFileCodeSet>>,
        #[serde(rename = "読み・字形による類推")]
        inference_by_reading_and_glyph: Option<Vec<MJFileCodeSet>>,
        #[serde(rename = "法務省告示582号別表第四")]
        moj_notice_582: Option<Vec<MJFileCodeSet>>,
        #[serde(rename = "法務省戸籍法関連通達・通知")]
        moj_family_register_act_related_notice: Option<Vec<MJFileCodeSet>>,
        #[serde(rename = "MJ文字図形名", deserialize_with = "deserialize_mj_code")]
        mj: MJCode,
    }

    #[derive(Deserialize)]
    struct MJFile {
        content: Vec<MJFileShrinkMapping>,
    }

    fn parse_uni_repr_set<'a>(
        codesets: impl IntoIterator<Item = &'a MJFileCodeSet>,
    ) -> Result<Vec<u32>, UnicodeReprParseError> {
        codesets
            .into_iter()
            .map(|cs| parse_uni_repr(&cs.ucs))
            .collect::<Result<Vec<_>, _>>()
    }

    let mut data: MJFile = serde_json::from_reader(BufReader::new(File::open(src)?))?;
    data.content.sort_by_key(|e| e.mj);

    let mut last = MJCode::new(0);
    let mut result = Vec::<MJShrinkMapping>::new();

    for entry in &data.content {
        let us = MJShrinkMappingUnicodeSet {
            jis_incorporation_ucs_unification_rule: entry
                .jis_incorporation_ucs_unification_rule
                .as_ref()
                .map(|v| parse_uni_repr_set(v))
                .transpose()?
                .unwrap_or_default(),
            inference_by_reading_and_glyph: entry
                .inference_by_reading_and_glyph
                .as_ref()
                .map(|v| parse_uni_repr_set(v))
                .transpose()?
                .unwrap_or_default(),
            moj_notice_582: entry
                .moj_notice_582
                .as_ref()
                .map(|v| parse_uni_repr_set(v))
                .transpose()?
                .unwrap_or_default(),
            moj_family_register_act_related_notice: entry
                .moj_family_register_act_related_notice
                .as_ref()
                .map(|v| parse_uni_repr_set(v))
                .transpose()?
                .unwrap_or_default(),
        };
        if us.is_empty() {
            continue;
        }
        // Fill reserved mappings if there are gaps
        result.extend((u32::from(last)..entry.mj.into()).map(|i| MJShrinkMapping {
            mj: MJCode::new(i),
            us: MJShrinkMappingUnicodeSet::default(),
        }));
        result.push(MJShrinkMapping { mj: entry.mj, us });
        last = entry.mj + 1;
    }
    Ok(result)
}

static CODE_TEMPLATE: &str = r#"use super::common_models::MenKuTen;

pub fn sm_uni_to_jis_mapping(mut state: i32, u: u32) -> (i32, Option<MenKuTen>) {
    loop {
        match state {
            0 => {
                match u {
                {%- for u, _ in uni_pairs_to_jis_mappings|items %}
                    {{u}} => break ({{ loop.index }}, None),
                {%- endfor %}
                    _ => break (0, None),
                }
            },
        {%- for u, inner in uni_pairs_to_jis_mappings|items %}
            {{ loop.index }} => {
                match u {
                {%- for u, j in inner|items %}
                    {{ u }} => break (0, Some(MenKuTen::from({{ j }}))),
                {%- endfor %}
                    _ => { state = 0; },
                }
            },
        {%- endfor %}
            _ => {},
        }
    }
}
"#;

lazy_static! {
    static ref JINJA_ENVIRONMENT: minijinja::Environment<'static> = {
        let mut env = minijinja::Environment::new();
        env.add_template("code", CODE_TEMPLATE)
            .expect("should add code template");
        env
    };
}

fn build_jis_reverse_mappings(
    cd: &mut inwire_models::ConversionData,
    gap_thr: u32,
) -> BTreeMap<u32, BTreeMap<u32, MenKuTen>> {
    let mut x = BTreeMap::<u32, MenKuTen>::new();
    let mut y = BTreeMap::<u32, BTreeMap<u32, MenKuTen>>::new();

    for m in &cd.jnta_mappings {
        if m.class == JISCharacterClass::Reserved {
            continue;
        }
        match m.us.len() {
            1 => {
                x.entry(m.us[0]).or_insert(m.jis);
                if !m.sus.is_empty() {
                    x.entry(m.sus[0]).or_insert(m.jis);
                }
            }
            2 => {
                y.entry(m.us[0])
                    .or_insert_with(BTreeMap::new)
                    .insert(m.us[1], m.jis);
            }
            _ => {}
        }
    }

    let mut s: Option<u32> = None;
    let mut e: u32 = 0;
    let mut chunk: Vec<MenKuTen> = Vec::new();
    for (&r, &i) in &x {
        if let Some(s_) = s {
            let g = r - e;
            if g >= gap_thr {
                cd.add_urange_to_jis_mapping(s_, std::mem::take(&mut chunk));
                s = Some(r);
            } else {
                chunk.extend((0..g).map(|_| MenKuTen::INVALID));
            }
        } else {
            s = Some(r);
        }
        chunk.push(i);
        e = r + 1
    }

    if let Some(s) = s {
        cd.add_urange_to_jis_mapping(s, std::mem::take(&mut chunk));
    }

    y
}

fn add_mj_mappings(cd: &mut inwire_models::ConversionData, mappings: Vec<MJMapping>) {
    let mut mj_mappings = Vec::with_capacity(mappings.len());
    for m in mappings {
        let r = cd.add_uivs_pool(m.v);
        mj_mappings.push(inwire_models::MJMapping {
            mj: m.mj,
            v: r.start,
        });
    }
    cd.mj_mappings = mj_mappings;
}

fn build_mj_reverse_mappings(cd: &mut inwire_models::ConversionData, gap_thr: u32) {
    let mut uni_to_mj_mappings: BTreeMap<u32, BTreeSet<MJCode>> = BTreeMap::new();

    for i in 0..cd.mj_mappings.len() - 1 {
        let m = &cd.mj_mappings[i];
        let uivps = &cd.uivs_pool[m.v as usize..cd.mj_mappings[i + 1].v as usize];
        for uivp in uivps {
            uni_to_mj_mappings
                .entry(uivp.u)
                .or_insert_with(BTreeSet::new)
                .insert(m.mj);
        }
    }

    let mut s: Option<u32> = None;
    let mut e: u32 = 0;
    let mut chunk: Vec<Vec<MJCode>> = Vec::new();

    for (u, ms) in uni_to_mj_mappings {
        if let Some(s_) = s {
            let g = u - e;
            if g >= gap_thr {
                cd.add_urange_to_mj_mapping(s_, std::mem::take(&mut chunk));
                s = Some(u);
            } else {
                chunk.extend((0..g).map(|_| Vec::new()));
            }
        } else {
            s = Some(u);
        }
        chunk.push(Vec::from_iter(ms.into_iter()));
        e = u + 1;
    }
    if let Some(s) = s {
        cd.add_urange_to_mj_mapping(s, chunk);
    }
}

fn add_mj_shrink_mappings(cd: &mut inwire_models::ConversionData, mappings: Vec<MJShrinkMapping>) {
    let mut mj_shrink_mappings = Vec::with_capacity(mappings.len());
    mj_shrink_mappings.extend(
        mappings
            .into_iter()
            .map(|m| inwire_models::MJShrinkMapping {
                mj: m.mj,
                us: inwire_models::MJShrinkMappingUnicodeSet {
                    jis_incorporation_ucs_unification_rule: cd
                        .add_uni_pool(m.us.jis_incorporation_ucs_unification_rule),
                    inference_by_reading_and_glyph: cd
                        .add_uni_pool(m.us.inference_by_reading_and_glyph),
                    moj_notice_582: cd.add_uni_pool(m.us.moj_notice_582),
                    moj_family_register_act_related_notice: cd
                        .add_uni_pool(m.us.moj_family_register_act_related_notice),
                },
            }),
    );
    cd.mj_shrink_mappings = mj_shrink_mappings;
}

fn do_jnta(
    dest_rs: impl AsRef<std::path::Path>,
    dest_bin: impl AsRef<std::path::Path>,
    src_jnta: impl AsRef<std::path::Path>,
    src_mj: impl AsRef<std::path::Path>,
    src_mj_shrink: impl AsRef<std::path::Path>,
    gap_thr: u32,
) -> Result<(), anyhow::Error> {
    eprintln!("reading {}...", src_jnta.as_ref().to_string_lossy());
    let jnta_mappings = read_jnta_excel_file(src_jnta)?;

    eprintln!("reading {}...", src_mj.as_ref().to_string_lossy());
    let mut mj_mappings = read_mj_excel_file(src_mj)?;
    mj_mappings.sort_by_key(|m| m.mj);

    eprintln!("reading {}...", src_mj_shrink.as_ref().to_string_lossy());
    let mj_shrink_mappings = read_mj_shrink_file(src_mj_shrink)?;

    let mut data = inwire_models::ConversionData::new(jnta_mappings);

    let rpm = build_jis_reverse_mappings(&mut data, gap_thr);
    add_mj_mappings(&mut data, mj_mappings);
    build_mj_reverse_mappings(&mut data, gap_thr);
    add_mj_shrink_mappings(&mut data, mj_shrink_mappings);
    data.finalize();

    eprintln!(
        "{} JNTAMappings, {} MJMappings, {} MJShrinkMappings.",
        data.jnta_mappings.len(),
        data.mj_mappings.len(),
        data.mj_shrink_mappings.len()
    );

    eprintln!("rendering code template...");
    let ctx = minijinja::context! {
        uni_pairs_to_jis_mappings => &rpm,
    };
    let tmpl = JINJA_ENVIRONMENT.get_template("code")?;
    let rendered = tmpl.render(ctx)?;
    std::fs::write(dest_rs, rendered)?;

    eprintln!("rendering binary...");
    {
        let buf = rkyv::to_bytes::<rkyv::rancor::Error>(&data)?;
        let mut compressed_buf =
            Vec::with_capacity(lz4_flex::block::get_maximum_output_size(buf.len()) + 4);
        compressed_buf.extend_from_slice((buf.len() as u32).to_le_bytes().as_slice());
        unsafe {
            let compressed_size = lz4_flex::compress_into(
                &buf,
                std::mem::transmute::<_, &mut [u8]>(compressed_buf.spare_capacity_mut()),
            )?;
            compressed_buf.set_len(compressed_size + 4);
        }
        std::fs::write(dest_bin, compressed_buf)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let out_dir = std::env::var("OUT_DIR")?;
    do_jnta(
        format!("{}/generated.rs", out_dir),
        format!("{}/generated.bin", out_dir),
        "target/data/jissyukutaimap1_0_0.xlsx",
        "target/data/mji.00602.xlsx",
        "target/data/MJShrinkMap.1.2.0.json",
        100,
    )?;
    Ok(())
}
