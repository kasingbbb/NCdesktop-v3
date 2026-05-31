pub mod library;
pub mod project;
pub mod asset;
pub mod timeline;
pub mod tag;
pub mod note;
pub mod search;
pub mod settings;
pub mod sync;
pub mod audio;
// 视频→音频提取命令（ffmpeg）。导入 mp4/mov 时自动抽音频；本命令亦供前端手动调用。
pub mod video_audio;
pub mod dropzone;
pub mod export;
pub mod llm;
pub mod workspace_folders;
pub mod knowledge_understanding;
pub mod knowledge;
pub mod knowledge_units;
pub mod knowledge_synthesis;
// 知识图谱（Step 9）：前端 KnowledgeGraphView 力导向图数据源。
// 此模块原先存在但未在 mod.rs 声明 → KnowledgeGraphView 启动时
// `Importing binding name 'getKnowledgeGraph' is not found` BLOCKER。
pub mod knowledge_graph;
pub mod conversion;
pub mod extraction;
pub mod outbound;
pub mod source_view;
// custom_prompt_v1 / task_002：用户自定义 Prompt 4 个 Tauri command。
pub mod user_prompt;
// custom_para_v1：PARA 自定义类目 CRUD command（PR-3 task_012 孤儿代码激活）。
// list/create/rename/disable/delete + add_alias 共 6 个 IPC，本期仅注册不上 UI。
pub mod categories;
// task_020：KC 集成层 3 个 Tauri command（get_kc_health / restart_kc_process / set_kc_settings）。
// 给前端 task_016 KcSettingsForm.tsx 提供 IPC 入口。
pub mod kc;
