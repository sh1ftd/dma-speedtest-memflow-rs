use clap::ValueEnum;
use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Debug)]
pub enum Connector {
    #[default]
    Pcileech,
    Native,
    Qemu, // QEMU virtual machine connector (experimental)
    Kvm,  // KVM virtual machine connector (experimental)
}

impl fmt::Display for Connector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Connector::Pcileech => write!(f, "pcileech"),
            Connector::Native => write!(f, "native"),
            Connector::Qemu => write!(f, "qemu"), // Not tested
            Connector::Kvm => write!(f, "kvm"),   // Not tested
        }
    }
}
