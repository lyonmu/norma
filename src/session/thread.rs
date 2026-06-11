#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionThread {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub updated_label: String,
}

pub fn sample_thread() -> SessionThread {
    SessionThread {
        id: "thread-design".to_string(),
        project_name: "norma".to_string(),
        title: "完善 Norma 项目设计".to_string(),
        updated_label: "14:32".to_string(),
    }
}
