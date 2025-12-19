
pub trait NormalizedName<T: ToString> {
    fn normalized(&self) -> String;
}
