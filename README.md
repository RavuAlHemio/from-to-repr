# from-to-repr

Provides the procedural macro `FromToRepr` which can be applied to a enum with an explicit representation (e.g `#[repr(u8)]`) and derives `TryFrom` from the representation type to the enum and `From` from the enum to the representation type.

As an example,

```rust
#[derive(FromToRepr)]
#[repr(u8)]
enum ColorChannel {
    RED = 0,
    GREEN = 1,
    BLUE = 2,
}
```

becomes

```rust
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
```

## from_to_other

Additionally, when the feature `from_to_other` is enabled, an attribute macro named `from_to_other` is enabled, which generates conversions to and from a base type, representing unknown values using an `Other` enum variant. For example:

```rust
use from_to_repr::from_to_other;

#[from_to_other(base_type = u8)]
enum ColorCommand {
    SetRed = 0,
    SetGreen = 1,
    SetBlue = 2,
    Other(u8),
}
```
is equivalent to
```rust
enum ColorCommand {
    SetRed,
    SetGreen,
    SetBlue,
    Other(u8),
}
impl ::core::convert::From<u8> for ColorCommand {
    fn from(base_value: u8) -> Self {
        if base_value == 0 {
            Self::SetRed
        } else if base_value == 1 {
            Self::SetGreen
        } else if base_value == 2 {
            Self::SetBlue
        } else {
            Self::Other(value)
        }
    }
}
impl ::core::convert::From<ColorCommand> for u8 {
    fn from(enum_value: ColorCommand) -> Self {
        match enum_value {
            ColorCommand::SetRed => 0,
            ColorCommand::SetGreen => 1,
            ColorCommand::SetBlue => 2,
            ColorCommand::Other(other) => other,
        }
    }
}
```
