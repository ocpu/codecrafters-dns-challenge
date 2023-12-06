#[macro_export]
macro_rules! define_type {
    {$vis:vis [$typ:ident, $qtyp:ident, $unknown:ident]: $($(#[$def_name:ident $($def_tt:tt)*])* $name:ident => $num:expr),* $(,)+ $($(#[$ext_def_name:ident $($ext_def_tt:tt)*])* [$ext_name:ident => $ext_num:expr]),* $(,)?} => {
       #[derive(Debug, Clone, Copy)]
        $vis enum $typ {
            $($(#[$def_name $($def_tt)*])* $name,)*
            $unknown(u16),
        }

        #[derive(Debug, Clone, Copy)]
        pub enum $qtyp {
            $($(#[$def_name $($def_tt)*])* $name,)*
            $($(#[$ext_def_name $($ext_def_tt)*])* $ext_name,)*
            $unknown(u16),
        }

        impl $typ {
            pub const fn as_u16(&self) -> u16 {
                match self {
                    $(Self::$name => $num,)*
                    Self::$unknown(val) => *val,
                }
            }
        }

        impl $qtyp {
            pub const fn as_u16(&self) -> u16 {
                match self {
                    $(Self::$name => $num,)*
                    $(Self::$ext_name => $ext_num,)*
                    Self::$unknown(val) => *val,
                }
            }
        }

        impl From<u16> for $typ {
            fn from(value: u16) -> Self {
                match value {
                    $($num => Self::$name,)*
                    val => Self::$unknown(val),
                }
            }
        }

        impl From<u16> for $qtyp {
            fn from(value: u16) -> Self {
                Self::from($typ::from(value))
            }
        }

        impl From<$typ> for $qtyp {
            fn from(value: $typ) -> Self {
                match value {
                    $($typ::$name => Self::$name,)*
                    $typ::$unknown(val) => match val {
                        $($ext_num => Self::$ext_name,)*
                        v => Self::$unknown(v),
                    }
                }
            }
        }
    };
}

pub use define_type;
