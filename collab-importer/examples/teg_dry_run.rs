//! Dry-run the NotionImporter against a real Notion-export zip and print the
//! resulting page hierarchy. Used to validate the May 2026 Notion-export
//! .md/folder collision fix without going through a full AppFlowy deploy.

use collab_importer::notion::NotionImporter;
use collab_importer::zip_tool::sync_zip::sync_unzip;
use std::env::temp_dir;
use std::path::PathBuf;

fn print_tree(page: &collab_importer::notion::page::NotionPage, depth: usize) {
  let indent = "  ".repeat(depth);
  let kind = if page.is_dir { "DIR " } else { "PAGE" };
  println!(
    "{}{} {} (children: {}, id: {})",
    indent,
    kind,
    page.notion_name,
    page.children.len(),
    page.notion_id.as_deref().unwrap_or("-"),
  );
  for c in &page.children {
    print_tree(c, depth + 1);
  }
}

#[tokio::main]
async fn main() {
  let zip = std::env::args()
    .nth(1)
    .expect("usage: teg_dry_run <path-to-notion.zip>");
  let zip_path = PathBuf::from(zip);

  let out_dir = temp_dir().join(uuid::Uuid::new_v4().to_string());
  std::fs::create_dir_all(&out_dir).expect("mkdir tmp");
  let name = zip_path
    .file_stem()
    .and_then(|s| s.to_str())
    .map(|s| s.to_string());
  let unzip = tokio::task::spawn_blocking({
    let out_dir = out_dir.clone();
    let zip_path = zip_path.clone();
    move || sync_unzip(zip_path, out_dir, name)
  })
  .await
  .expect("join")
  .expect("unzip");

  println!("[unzip] -> {:?}", unzip.unzip_dir);

  let workspace_id = uuid::Uuid::new_v4();
  let host = "http://test.appflowy.cloud".to_string();
  let importer =
    NotionImporter::new(1, &unzip.unzip_dir, workspace_id, host).expect("create importer");

  let info = importer.import().await.expect("import");

  println!(
    "=== Workspace: {} ({}) ===",
    info.name, info.workspace_id
  );
  println!("Top-level views: {}", info.views().len());
  println!();

  for view in info.views() {
    print_tree(view, 0);
  }
}
