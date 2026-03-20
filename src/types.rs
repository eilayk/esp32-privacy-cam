pub trait JpegImage {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn data(&self) -> &[u8];
    fn length(&self) -> usize {
        self.data().len()
    }
}
