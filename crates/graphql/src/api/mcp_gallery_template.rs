#[derive(Debug, Clone)]
pub struct MCPTemplateVariable {
    pub key: String,
    pub allowed_values: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct MCPJsonTemplate {
    pub json: String,
    pub variables: Vec<MCPTemplateVariable>,
}

#[derive(Debug, Clone)]
pub struct MCPGalleryTemplate {
    pub description: String,
    pub gallery_item_id: String,
    pub instructions_in_markdown: Option<String>,
    pub json_template: MCPJsonTemplate,
    pub template: String,
    pub title: String,
    pub version: i32,
}
