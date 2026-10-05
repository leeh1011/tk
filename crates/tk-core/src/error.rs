use rusqlite;

#[derive(Debug,thiserror::Error)]
pub enum Error{
    #[error("내용을 입력해주세요")]
    EmptyNote,
    #[error("serer error")]
    Db(#[from] rusqlite::Error),
}

pub type Result<T>=std::result::Result<T,Error>;