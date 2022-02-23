# from-to-repr

Provides the procedural macro `FromToRepr` which can be applied to a enum with an explicit representation (e.g `#[repr(u8)]`) and derives `TryFrom` from the representation type to the enum and `From` from the enum to the representation type.

As an example,

    #[derive(FromToRepr)]
    #[repr(u8)]
    enum ColorChannel {
        RED = 0,
        GREEN = 1,
        BLUE = 2,
    }

becomes

    #[repr(u8)]
    enum ColorChannel {
        RED = 0,
        GREEN = 1,
        BLUE = 2,
    }
    impl ::std::convert::TryFrom<u8> for ColorChannel {
        type Error = u8;
        fn try_from(value: u8) -> Result<Self, Self::Error> {
            if value == 0 {
                Ok(Self::RED)
            } else if value == 1 {
                Ok(Self::GREEN)
            } else if value == 2 {
                Ok(Self::BLUE)
            } else {
                Err(value)
            }
        }
    }
    impl ::std::convert::From<ColorChannel> for u8 {
        fn from(value: ColorChannel) -> Self {
            match value {
                ColorChannel::RED => 0,
                ColorChannel::GREEN => 1,
                ColorChannel::BLUE => 2,
            }
        }
    }
