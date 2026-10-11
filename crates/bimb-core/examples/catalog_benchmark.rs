use bimb_core::{catalog::Catalog, model::Query};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let catalog = Catalog::open(&dir.path().join("catalog.sqlite"))?;
    catalog.call(|c|{
        let tx=c.transaction()?;
        tx.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES('s','Benchmark',X'2F','/','v','r')",[])?;
        for n in 0..3000 {
            let id=format!("asset-{n:04}");let name=format!("Holiday mountain {n:04}.jpg");
            tx.execute("INSERT INTO assets(id,source_id,path,filename,folder,file_key,revision,kind,size,modified,added,seen,captured) VALUES(?,'s',?,?,'Holiday',?,'r','photo',10000000,'2024-01-01','2026-01-01',1,'2024-01-01')",rusqlite::params![id,id.as_bytes(),name,id])?;
        }tx.commit()?;Ok(())
    })?;
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let page = catalog.query(Query {
            text: "mountain".into(),
            ..Default::default()
        })?;
        assert_eq!(page.total, 3000);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({"dataset":"3000 synthetic catalog rows, no media IO","samples":100,"search_p50_ms":times[49],"search_p95_ms":times[94],"sqlite":rusqlite::version()})
    );
    Ok(())
}
