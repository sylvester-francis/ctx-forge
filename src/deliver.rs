//! Delivery choices — the destination options for the crafted prompt.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverChoice {
    PipeClaude,
    PipeAgent,
    PipeGemini,
    CopyMarkdown,
    CopyXml,
    CopyJson,
    Export,
    ExportXml,
    ExportJson,
}

impl DeliverChoice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PipeClaude => "pipe claude (XML)",
            Self::PipeAgent => "pipe agent (markdown)",
            Self::PipeGemini => "pipe gemini (markdown)",
            Self::CopyMarkdown => "copy markdown",
            Self::CopyXml => "copy XML",
            Self::CopyJson => "copy JSON",
            Self::Export => "export markdown to stdout",
            Self::ExportXml => "export XML to stdout",
            Self::ExportJson => "export JSON to stdout",
        }
    }

    pub fn all() -> &'static [DeliverChoice] {
        &[
            Self::PipeClaude,
            Self::PipeAgent,
            Self::PipeGemini,
            Self::CopyMarkdown,
            Self::CopyXml,
            Self::CopyJson,
            Self::Export,
            Self::ExportXml,
            Self::ExportJson,
        ]
    }
}
