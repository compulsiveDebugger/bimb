use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub path: String,
    pub volume_id: String,
    pub state: String,
    pub scan_state: String,
    pub total: i64,
    pub errors: i64,
    pub media_errors: i64,
    pub scan_errors: i64,
    pub last_scan: Option<String>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceIssue {
    pub id: i64,
    pub path: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceIssuePage {
    pub issues: Vec<SourceIssue>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
    pub captured: Option<String>,
    pub offset: Option<String>,
    pub camera: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_raw: Option<String>,
    pub metadata_state: String,
    pub thumbnail_state: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub filename: String,
    pub folder: String,
    pub kind: String,
    pub size: i64,
    pub modified: String,
    pub added: String,
    pub captured: Option<String>,
    pub date_source: String,
    pub effective_date: String,
    pub revision: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
    pub camera: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub metadata_state: String,
    pub thumbnail_state: String,
    pub error: Option<String>,
    pub availability: String,
    pub favorite: bool,
    pub caption: String,
    pub tags: Vec<String>,
    pub place: String,
    pub date_override: String,
    pub exclude_memory: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Annotation {
    pub favorite: bool,
    pub caption: String,
    pub tags: Vec<String>,
    pub place: String,
    pub date_override: String,
    pub exclude_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Query {
    pub text: String,
    pub source: String,
    pub kind: String,
    pub favorites: bool,
    pub availability: String,
    pub date_from: String,
    pub date_to: String,
    pub place: String,
    pub folder: String,
    pub album: String,
    pub memory: bool,
    pub errors: bool,
    pub gps: bool,
    pub sort: String,
    pub ascending: bool,
    pub cursor: Option<String>,
    pub limit: usize,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            text: String::new(),
            source: String::new(),
            kind: String::new(),
            favorites: false,
            availability: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            place: String::new(),
            folder: String::new(),
            album: String::new(),
            memory: false,
            errors: false,
            gps: false,
            sort: "capture".into(),
            ascending: false,
            cursor: None,
            limit: 120,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    pub assets: Vec<Asset>,
    pub total: i64,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub count: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub query: Query,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub thumbnail_budget: u64,
    pub memories_enabled: bool,
    pub excluded_dates: Vec<[String; 2]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            thumbnail_budget: 1024 * 1024 * 1024,
            memories_enabled: true,
            excluded_dates: vec![],
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Overview {
    pub sources: Vec<Source>,
    pub albums: Vec<Album>,
    pub collections: Vec<Collection>,
    pub settings: Settings,
    pub total: i64,
    pub photos: i64,
    pub videos: i64,
    pub favorites: i64,
    pub bytes: i64,
    pub thumbnail_bytes: u64,
    pub folders: Vec<String>,
    pub places: Vec<String>,
}
