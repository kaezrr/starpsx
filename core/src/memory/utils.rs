pub trait ByteAddressable {
    type Bytes: for<'a> TryFrom<&'a [u8], Error: core::fmt::Debug> + AsRef<[u8]>;
    fn from_le_bytes(bytes: Self::Bytes) -> Self;
    fn to_le_bytes(self) -> Self::Bytes;
}

macro_rules! int_impl {
    ($int:ty) => {
        impl ByteAddressable for $int {
            type Bytes = [u8; size_of::<Self>()];
            fn from_le_bytes(bytes: Self::Bytes) -> Self {
                <$int>::from_le_bytes(bytes)
            }
            fn to_le_bytes(self) -> Self::Bytes {
                self.to_le_bytes()
            }
        }
    };
}

int_impl!(u8);
int_impl!(u16);
int_impl!(u32);
