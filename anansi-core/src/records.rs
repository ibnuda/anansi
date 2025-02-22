use std::ops::DerefMut;
use std::marker::PhantomData;
use std::num::ParseIntError;
use std::result;
use std::fmt;
use std::str::FromStr;
use std::ops::Deref;
use std::error::Error;
use async_trait::async_trait;

use sqlx::{Type, Decode, Database, database::HasValueRef};
use rand::Rng;

use crate::web::{BaseRequest, Parameters, Result};
use crate::db::{Db, DbRow, DbRowVec, DbPool, DbTypeInfo, invalid, escape, Count, Whose, WhoseArg, DeleteWhose, OrderBy, OrderByArg, Limit};
use crate::admin_site::AdminField;
pub use crate::datetime::DateTime;

#[macro_export]
macro_rules! get_or_404 {
    ($record:path, $req:ident) => {
        match $req.get_record::<$record>().await {
            Ok(m) => m,
            Err(_) => {
                return Err(Box::new(anansi::web::Http404::from($req)))
            },
        }
    }
}

macro_rules! impl_decode {
    ($d:ty, $t:ty) => {
        impl Type<Db> for $d {
            fn type_info() -> DbTypeInfo {
                <$t as Type<Db>>::type_info()
            }
        }
        impl<'r, DB: Database> Decode<'r, DB> for $d
        where $t: Decode<'r, DB> {
            fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> result::Result<$d, Box<dyn Error + 'static + Send + Sync>> {
                let value = <$t as Decode<DB>>::decode(value)?;
                Ok(Self::from_val(value).unwrap())
            }
        }
    }
}

impl<const N: u16> AdminField for VarChar<N> {
    fn admin_field(&self) -> String {
        self.to_string()
    }
}

impl AdminField for Text {
    fn admin_field(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Boolean {
    b: bool,
}

impl Boolean {
    pub fn new(b: bool) -> Self {
        Self {b}
    }
    pub fn from(s: &str) -> Result<Self> {
            let b = match s {
                "false" => false,
                "0" => false,
                "true" => true,
                "1" => true,
                _ => return Err(invalid()),
            };
            Ok(Self{b})
    }
    pub fn field() -> RecordField {
        RecordField::new("boolean".to_string())
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.b)
    }
}

impl PartialEq<bool> for Boolean {
    fn eq(&self, other: &bool) -> bool {
        self.b == *other
    }
}

impl DataType for Boolean {
    type T = bool;

    fn from_val(b: bool) -> Result<Self> {
        Ok(Self{b})
    }
}

impl ToSql for Boolean {
    fn to_sql(&self) -> String {
        format!("{}", self.b)
    }
}

impl_decode!(Boolean, bool);

#[derive(Clone, PartialEq, Copy, Debug)]
pub struct BigInt {
    n: i64,
}

impl BigInt {
    pub fn new(n: i64) -> Self {
        Self {n}
    }
    pub fn from(s: &str) -> result::Result<Self, ParseIntError> {
        match s.parse() {
            Ok(n) => Ok(Self {n}),
            Err(e) => Err(e),
        }
    }
    pub fn as_i64(&self) -> i64 {
        self.n
    }
    pub fn into(self) -> i64 {
        self.n
    }
    pub fn field() -> RecordField {
        RecordField::new("bigint".to_string())
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.n)
    }
}

impl DataType for BigInt {
    type T = i64;

    fn from_val(n: i64) -> Result<Self> {
        Ok(Self {n})
    }
}

impl ToSql for BigInt {
    fn to_sql(&self) -> String {
        format!("{}", self.n)
    }
}

impl Type<Db> for BigInt {
    fn type_info() -> DbTypeInfo {
        <i64 as Type<Db>>::type_info()
    }
}

impl<'r, DB: Database> Decode<'r, DB> for BigInt
where i64: Decode<'r, DB> {
    fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> result::Result<BigInt, Box<dyn Error + 'static + Send + Sync>> {
        let value = <i64 as Decode<DB>>::decode(value)?;
        Ok(Self::from_val(value).unwrap())
    }
}

impl PartialEq<i64> for BigInt {
    fn eq(&self, other: &i64) -> bool {
        self.n == *other
    }
}

pub fn generate_id() -> BigInt {
    let mut rng = rand::thread_rng();
    BigInt::new(rng.gen_range(0..i64::MAX))
}

#[derive(Clone, Debug)]
pub struct Text {
    s: String,
}

impl Text {
    pub fn new() -> Self {
        Self {s: String::new()}
    }
    pub fn from(s: String) -> Self {
        Self {s}
    }
    pub fn field() -> RecordField {
        RecordField::new("text".to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.s
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.s == *other
    }
}

impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        self.s == *other
    }
}

impl DataType for Text {
    type T = String;

    fn from_val(s: String) -> Result<Self> {
        Ok(Self {s})
    }
}

impl ToSql for Text {
    fn to_sql(&self) -> String {
        escape(&self.s)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.s)
    }
}

impl Deref for Text {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.s
    }
}

impl Type<Db> for Text {
    fn type_info() -> DbTypeInfo {
        <String as Type<Db>>::type_info()
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Text
where String: Decode<'r, DB> {
    fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> result::Result<Text, Box<dyn Error + 'static + Send + Sync>> {
        let value = <String as Decode<DB>>::decode(value)?;
        Ok(Self::from_val(value).unwrap())
    }
}

#[derive(Clone, Debug)]
pub struct VarChar<const N: u16> {
    s: String,
}

impl<const N: u16> VarChar<N> {
    pub fn new() -> Self {
        Self {s: String::new()}
    }
    pub fn from(s: String) -> Result<Self> {
        Self::from_val(s)
    }
    pub fn as_str(&self) -> &str {
        &self.s
    }    
    pub fn field() -> RecordField {
        RecordField::new(format!("varchar({})", N))
    }
}

impl<const N: u16> FromStr for VarChar<N> {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_val(s.to_string())
    }
}

impl FromStr for Text {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_val(s.to_string())
    }
}

impl<const N: u16> PartialEq<&str> for VarChar<N> {
    fn eq(&self, other: &&str) -> bool {
        self.s == *other
    }
}

impl<'a, const N: u16> PartialEq<String> for &'a VarChar<N> {
    fn eq(&self, other: &String) -> bool {
        self.s == *other
    }
}

impl<const N: u16> DataType for VarChar<N> {
    type T = String;

    fn from_val(s: String) -> Result<Self> {
        if s.len() > N as usize {
            Err(invalid())
        } else {
            Ok(Self {s})
        }
    }
}

impl<const N: u16> ToSql for VarChar<N> {
    fn to_sql(&self) -> String {
        escape(&self.s)
    }
}

impl<'a, const N: u16> ToSql for &'a VarChar<N> {
    fn to_sql(&self) -> String {
        escape(&self.s)
    }
}

impl<const N: u16> fmt::Display for VarChar<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.s)
    }
}

impl<const N: u16> Deref for VarChar<N> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.s
    }
}

impl<const N: u16> Type<Db> for VarChar<N> {
    fn type_info() -> DbTypeInfo {
        <String as Type<Db>>::type_info()
    }
}

impl<'r, DB: Database, const N: u16> Decode<'r, DB> for VarChar<N>
where String: Decode<'r, DB> {
    fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> result::Result<VarChar<N>, Box<dyn Error + 'static + Send + Sync>> {
        let value = <String as Decode<DB>>::decode(value)?;
        Ok(Self::from_val(value).unwrap())
    }
}

#[derive(Debug)]
pub struct ForeignKey<M: Record, O: OnDelete = Cascade> {
    pk: M::Pk,
    o: PhantomData<O>,
}

impl<M: Record, O: OnDelete> Clone for ForeignKey<M, O> {
    fn clone(&self) -> Self {
        Self {
            pk: self.pk.clone(),
            o: PhantomData,
        }
    }
}

impl<M: Record, O: OnDelete> ForeignKey<M, O> {
    pub fn new(m: &M) -> Self {
        Self {pk: m.pk().clone(), o: PhantomData}
    }
    pub fn from_data(t: <M as Record>::Pk) -> Result<Self> {
        Ok(Self {pk: t, o: PhantomData})
    }
    pub fn pk(&self) -> M::Pk {
        self.pk.clone()
    }
    pub fn eq_record(&self, m: &M) -> bool where <M as Record>::Pk: PartialEq {
        self.pk == m.pk()
    }
    pub async fn get<B: BaseRequest>(&self, req: &B) -> Result<M> {
        M::find(self.pk()).get(req).await
    }
    pub async fn raw_get(&self, pool: &DbPool) -> Result<M> {
        M::find(self.pk()).raw_get(pool).await
    }
}

impl<M: Record, O: OnDelete> DataType for ForeignKey<M, O> where M::Pk: std::fmt::Display {
    type T = <<M as Record>::Pk as DataType>::T;

    fn from_val(t: <<M as Record>::Pk as DataType>::T) -> Result<Self> {
        Ok(Self {pk: M::Pk::from_val(t)?, o: PhantomData})
    }
}

impl<M: Record, O: OnDelete> ToSql for ForeignKey<M, O> where M::Pk: std::fmt::Display {
    fn to_sql(&self) -> String {
        self.pk.to_sql()
    }
}

impl<M: Record, O: OnDelete> DataType for Option<ForeignKey<M, O>> where M::Pk: std::fmt::Display {
    type T = Option<<<M as Record>::Pk as DataType>::T>;

    fn from_val(t: Option<<<M as Record>::Pk as DataType>::T>) -> Result<Self> {
        if let Some(s) = t {
            Ok(Some(ForeignKey::from_val(s)?))
        } else {
            Ok(None)
        }
    }
}

impl<M: Record, O: OnDelete> ToSql for Option<ForeignKey<M, O>> where M::Pk: std::fmt::Display {
    fn to_sql(&self) -> String {
        if let Some(s) = self {
            s.to_sql()
        } else {
            "NULL".to_string()
        }
    }
}

impl<M: Record, O: OnDelete> fmt::Display for ForeignKey<M, O> where M::Pk: std::fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pk)
    }
}

impl<'r, M: Record, O: OnDelete> Type<Db> for ForeignKey<M, O>
where <<M as Record>::Pk as DataType>::T: Decode<'r, Db> + Type<Db> {
    fn type_info() -> DbTypeInfo {
        <<<M as Record>::Pk as DataType>::T as Type<Db>>::type_info()
    }
}

impl<'r, M: Record, O: OnDelete, DB: Database> Decode<'r, DB> for ForeignKey<M, O>
where <<M as Record>::Pk as DataType>::T: Decode<'r, DB>, <M as Record>::Pk: std::fmt::Display {
    fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> result::Result<ForeignKey<M, O>, Box<dyn Error + 'static + Send + Sync>> {
        let value = <<<M as Record>::Pk as DataType>::T as Decode<DB>>::decode(value)?;
        Ok(Self::from_val(value).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct ManyToMany<M: Record> {
    m: PhantomData<M>,
}

impl<M: Record> ManyToMany<M> {
    pub fn new() -> Self {
        Self {m: PhantomData}
    }
}

pub struct Objects<M>(Vec<M>);

impl<M: Record> Objects<M> {
    pub fn pks(&self) -> Vec<M::Pk> {
        let mut v = vec![];
        for d in &self.0 {
            v.push(d.pk());
        }
        v
    }
    pub async fn parents<P: Record, O: OnDelete, B: BaseRequest>(&self, req: &B, f: for<'a> fn(&'a M) -> &'a ForeignKey<P, O>) -> Result<Objects<P>> {
        if self.0.is_empty() {
            return Ok(Objects::<P>::new());
        }
        let mut v = vec![];
        for d in &self.0 {
            v.push(f(d).pk());
        }
        P::find_in(&v).query(req).await
    }
}

impl<M: Record> Objects<M> {
    pub fn new() -> Self {
        Self {0: vec![]}
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn push(&mut self, value: M) {
        self.0.push(value);
    }
    pub fn pop(&mut self) -> Option<M> {
        self.0.pop()
    }
    pub fn swap_remove(&mut self, index: usize) -> M {
        self.0.swap_remove(index)
    }
}

impl<M: Record> IntoIterator for Objects<M> {
    type Item = M;
    type IntoIter = std::vec::IntoIter<M>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<M: Record> Deref for Objects<M> {
    type Target = Vec<M>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M: Record> DerefMut for Objects<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait OnDelete {
    fn on_delete() -> &'static str;
}

#[derive(Debug)]
pub struct Cascade {}

impl OnDelete for Cascade {
    fn on_delete() -> &'static str {
        "CASCADE"
    }
}

mod private {
    use super::*;
    use super::super::datetime::DateTime;
    pub trait Sealed {}
    impl Sealed for BigInt {}
    impl Sealed for Boolean {}
    impl Sealed for Text {}
    impl Sealed for DateTime {}
    impl<const N: u16> Sealed for VarChar<N> {}
    impl<'a, const N: u16> Sealed for &'a VarChar<N> {}
    impl<M: Record, O: OnDelete> Sealed for ForeignKey<M, O> {}
    impl Sealed for i64 {}
    impl Sealed for bool {}
    impl Sealed for String {}
    impl Sealed for &String {}
    impl<'a> Sealed for &'a str {}
    impl<const N: u16> Sealed for Option<VarChar<N>> {}
    impl Sealed for Option<Text> {}
    impl<M: Record, O: OnDelete> Sealed for Option<ForeignKey<M, O>> {}
}

#[derive(Clone)]
pub struct RecordField {
    ty: String,
    primary_key: bool,
    unique: bool,
    null: bool,
    constraints: Vec<String>,
}

impl RecordField {
    pub fn new(ty: String) -> Self {
        Self {ty, primary_key: false, unique: false, null: false, constraints: vec![]}
    }
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    pub fn null(mut self) -> Self {
        self.null = true;
        self
    }
    pub fn foreign_key(mut self, app_name: &'static str, other_name: &'static str, pk_name: &'static str) -> Self {
        self.constraints.push(format!("FOREIGN KEY (\"{}\")", other_name));
        self.constraints.push(format!("REFERENCES \"{}_{}\" (\"{}\")", app_name, other_name, pk_name));
        self.constraints.push("ON DELETE CASCADE".to_string());
        self
    }
    pub fn to_syntax(&self) -> (String, Vec<String>) {
        let mut s = format!("{}", self.ty);
        if !self.null {
            s.push_str(" NOT NULL");
        }
        if self.primary_key {
            s.push_str(" PRIMARY KEY");
        }
        if self.unique {
            s.push_str(" UNIQUE");
        }
        (s, self.constraints.clone())
    }
}

impl<const N: u16> DataType for Option<VarChar<N>> {
    type T = Option<String>;

    fn from_val(t: Self::T) -> Result<Self> {
        if let Some(s) = t {
            Ok(Some(VarChar::from_val(s)?))
        } else {
            Ok(None)
        }
    }
}

impl<const N: u16> ToSql for Option<VarChar<N>> {
    fn to_sql(&self) -> String {
        if let Some(s) = self {
            s.to_sql()
        } else {
            "NULL".to_string()
        }
    }
}

impl DataType for Option<Text> {
    type T = Option<String>;

    fn from_val(t: Self::T) -> Result<Self> {
        if let Some(s) = t {
            Ok(Some(Text::from_val(s)?))
        } else {
            Ok(None)
        }
    }
}

impl ToSql for Option<Text> {
    fn to_sql(&self) -> String {
        if let Some(s) = self {
            s.to_sql()
        } else {
            "NULL".to_string()
        }
    }
}

impl<'a> ToSql for &'a str {
    fn to_sql(&self) -> String {
        escape(self)
    }
}

impl ToSql for String {
    fn to_sql(&self) -> String {
        escape(self)
    }
}

impl ToSql for &String {
    fn to_sql(&self) -> String {
        escape(self)
    }
}

pub trait DataType: Clone + ToSql + private::Sealed {
    type T;
    fn from_val(t: Self::T) -> Result<Self> where Self: Sized;
}

pub trait ToSql: private::Sealed {
    fn to_sql(&self) -> String;
}

#[async_trait]
pub trait FromParams: Record {
    fn pk_from_params(params: &Parameters) -> Result<BigInt>;
    async fn from_params(params: &Parameters) -> Result<Whose<Self>> where Self: Sized;
}

#[async_trait]
pub trait RecordTuple<B: BaseRequest> {
    async fn check(object_namespace: Text, object_key: BigInt, object_predicate: Text, req: &B) -> Result<()>;
}

#[async_trait]
pub trait Relate<B: BaseRequest> {
    async fn on_save(&self, _req: &B) -> Result<()> where Self: Sized {
        Ok(())
    }
    async fn on_delete(&self, _req: &B) -> Result<()> where Self: Sized {
        Ok(())
    }
}

pub trait ToUrl {
    fn to_url(&self) -> String;
}

#[async_trait]
pub trait Record: Sized {
    type Pk: DataType;
    const NAME: &'static str;
    const PK_NAME: &'static str;
    fn pk(&self) -> Self::Pk;
    fn pk_mut(&mut self) -> &mut Self::Pk;
    fn table_name() -> String;
    fn find(data: Self::Pk) -> Whose<Self> where Self: Sized;
    fn find_in(keys: &Vec<Self::Pk>) -> Limit<Self> where Self: Sized;
    fn count() -> Count<Self>;
    fn whose(w: WhoseArg<Self>) -> Whose<Self> where Self: Sized;
    fn delete_whose(w: WhoseArg<Self>) -> DeleteWhose<Self> where Self: Sized;
    fn limit(n: u32) -> Limit<Self> where Self: Sized;
    fn get(row: DbRow) -> Result<Self> where Self: Sized;
    fn from(rows: DbRowVec) -> Result<Objects<Self>> where Self: Sized;
    fn order_by(w: OrderByArg<Self>) -> OrderBy<Self> where Self: Sized;
    async fn update<B: BaseRequest>(&mut self, req: &B) -> Result<()> where Self: Sized;
    async fn raw_update(&mut self, pool: &DbPool) -> Result<()> where Self: Sized;
    async fn delete<B: BaseRequest>(&self, req: &B) -> Result<()> where Self: Sized;
    async fn save<B: BaseRequest>(self, req: &B) -> Result<Self> where Self: Sized;
    async fn raw_save(self, pool: &DbPool) -> Result<Self> where Self: Sized;
}
