use egui::{
    CentralPanel, FontData, FontDefinitions, FontId, Label, RichText, ScrollArea, Visuals, Window,
};
use sqlx::{MySql, Pool};

use crate::mysql_db::{query_data, SslDataRow};

// 异步显示数据的函数，假设此函数在一个Tokio的异步环境中被调用
pub async fn display_data_async(pool: &Pool<MySql>) {
    // 查询数据，这里直接在异步上下文中调用异步函数
    let data: Vec<SslDataRow> = match query_data(&pool).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error querying data: {}", e);
            return;
        }
    };

    // 初始化eframe并启动应用
    // 注意：此处简化展示，实际应用中需考虑如何与现有异步架构整合
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SSL Data Viewer",
        options,
        Box::new(move |_cc| {
            // 确保selected_id字段被初始化
            Box::new(MyApp::new(data))
        }),
    )
    .expect("Failed to run eframe application");
}

struct MyApp {
    data: Vec<SslDataRow>,
    expanded_id: Option<i64>, // 选中的ID，用于展示完整buf
    fonts: FontDefinitions,   // 添加这个字段来存储字体定义
}
impl MyApp {
    fn new(data: Vec<SslDataRow>) -> Self {
        // 初始化字体定义，加载自定义字体
        let mut fonts = FontDefinitions::default();
        if let Ok(font_data) = std::fs::read("./LXGWWenKai-Bold.ttf") {
            let font = FontData::from_owned(font_data);
            fonts.font_data.insert("my_font".to_owned(), font);

            // 配置字体族以应用自定义字体
            fonts
                .families
                .insert(egui::FontFamily::Proportional, vec!["my_font".to_owned()]);
            fonts
                .families
                .insert(egui::FontFamily::Monospace, vec!["my_font".to_owned()]);
        } else {
            eprintln!("Custom font file not found at specified path.");
        }

        Self {
            data,
            expanded_id: None,
            fonts,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let text_size = 18.0;
        // 应用自定义字体
        ctx.set_fonts(self.fonts.clone());

        // 设置界面为亮色主题
        ctx.set_visuals(Visuals::light());

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for row in &self.data {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("ID: {}", row.id))
                                .font(egui::FontId::monospace(text_size)),
                        );
                        ui.label(
                            egui::RichText::new(format!("Time: {}", row.timestamp))
                                .font(egui::FontId::monospace(text_size)),
                        );
                        ui.label(
                            egui::RichText::new(format!("PID: {}", row.pid))
                                .font(egui::FontId::monospace(text_size)),
                        );
                        ui.label(
                            egui::RichText::new(format!("Command: {}", row.comm))
                                .font(egui::FontId::monospace(text_size)),
                        );

                        // 添加横线分隔
                        ui.separator();

                        // 修改按钮逻辑，记录点击的ID
                        if ui.button("Expand").clicked() {
                            self.expanded_id = Some(row.id);
                        }
                        // 显示buf的预览（例如前50个字符）
                        let preview_buf = if row.buf.chars().count() > 50 {
                            format!(
                                "{}...",
                                &row.buf[..row.buf.chars().take(50).collect::<String>().len()]
                            )
                        } else {
                            row.buf.clone() // 如果少于50个字符，直接使用原始字符串
                        };

                        ui.label(
                            egui::RichText::new(preview_buf)
                                .font(egui::FontId::monospace(text_size)), // 这里设置了18号字体作为示例
                        );
                    });

                    // 在每个item后添加额外的间距作为视觉上的分隔
                    ui.add_space(10.0);
                }
            });

            // 展示完整buf的弹窗
            if let Some(selected_id) = self.expanded_id {
                // 确保找到对应的完整buf并展示
                if let Some(row) = self.data.iter().find(|row| row.id == selected_id) {
                    Window::new(format!("Full Buffer - ID: {}", selected_id))
                        .collapsible(true)
                        .resizable(true)
                        .show(ctx, |ui| {
                            let full_buf = &row.buf; // 直接获取row的buf字段
                                                     // 使用TextEdit以支持文本自动换行和界面自适应
                            let full_buf_clone = full_buf.clone(); // 克隆buf以用于展示，避免直接修改原数据
                                                                   // ScrollArea::both().show(ui, |ui| {
                                                                   //     // let font_size = 100.00; // 设定你希望的字体大小
                                                                   //     ui.add(egui::TextEdit::multiline(&mut full_buf_clone) // Use the clone here
                                                                   //         // .font(egui::TextStyle::Monospace)
                                                                   //         .font(egui::FontId::monospace(30.0))
                                                                   //         // .font(egui::FontId::new(100.00, egui::FontFamily::Name("my_font".into())))
                                                                   //         .code_editor()
                                                                   //         .interactive(false) // The original string won't be modified anyway
                                                                   //         .desired_width(f32::INFINITY)
                                                                   //     );
                                                                   // });

                            ScrollArea::both().show(ui, |ui| {
                                // 假设你已经有了一个包含多行文本的字符串 full_buf_clone
                                let wrapped_text = full_buf_clone
                                    .split('\n')
                                    .map(RichText::new)
                                    .collect::<Vec<_>>();

                                // 自定义字体大小
                                let font_size = 18.0;
                                let font_id = FontId::new(font_size, egui::FontFamily::Monospace);

                                // 使用 RichText 和多个 Label 来展示每一行文本
                                for line in wrapped_text {
                                    ui.add(Label::new(line.clone().font(font_id.clone())));
                                    // 添加换行
                                    ui.add(egui::Separator::default().spacing(0.0));
                                }
                            });

                            // 添加关闭按钮，点击后关闭窗口
                            if ui.button(RichText::new("Close").size(15.0)).clicked() {
                                self.expanded_id = None; // 关闭窗口时清除expanded_id
                            }
                        });
                }
            }
        });
    }
}
