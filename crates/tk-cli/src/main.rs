use std::path::PathBuf;
use anyhow::Context;
use clap::{Parser,Subcommand};
use directories::ProjectDirs;
use tk_core::{Note, Store};

#[derive(Parser,Debug)]
#[command(name="tk",version,about="personal toolkit")]
struct Cli{
    #[command(subcommand)]
    command:Command,
    #[arg(long,env="TK_DB",global=true)]
    /// DB 파일 경로 (기본값: OS 데이터 폴더/tk.db)
    db:Option<PathBuf>
}

#[derive(Subcommand,Debug)]

enum Command{
    /// 메모 관리
    Note{
        #[command(subcommand)]
        action:NoteAction,
    },
}

#[derive(Subcommand,Debug)]
enum NoteAction{
    /// 메모 추가
    Add{
        #[arg(required=true)]
        /// 메모 내용
        text:Vec<String>},
    /// 최근 메모 리스트
    List{
        #[arg(short, long, default_value_t = 10)]
        /// 출력 메모 개수
        limit:usize},
    /// 키워드로 메모 검색
    Search{
        ///검색 키워드
        keyword:String},
    ///메모 삭제
    Rm{
        ///삭제 메모 아이디
        id:i64},
}

fn default_db_path()->anyhow::Result<PathBuf>{
    let dirs=ProjectDirs::from("", "", "tk").context("...")?;
    let dir=dirs.data_dir();
    std::fs::create_dir_all(dir)?;
    Ok(dir.join("tk.db"))
}

fn run_notes(store:&Store,action:NoteAction)->anyhow::Result<()>{
    match action{
        NoteAction::Add { text }=>{store.add_note(text);},
        NoteAction::List { limit }=>{store.list_notes(limit)},
        NoteAction::Search { keyword }=>{},
        NoteAction::Rm { id }=>{}
    }
    Ok(())
}

fn main()->anyhow::Result<()>{
    let cli=Cli::parse();
    let db_path=match cli.db{
        Some(path)=>path,
        None=>default_db_path()?,

    };
    Store::open(&db_path).with_context(||format!("can't open db: {}]",db_path.display()))?;

    match cli.command{
        Command::Note { action }=>run_note(&store,action)?,
        
    }
    Ok(())
}