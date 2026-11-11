// Hand-maintained prost bindings matching proto/transfers.proto.
// `substreams build` / protogen may regenerate these; keep the schema in sync.

pub mod solana {
    pub mod transfers {
        pub mod v1 {
            #[derive(Clone, PartialEq, ::prost::Message)]
            pub struct Transfers {
                #[prost(message, repeated, tag = "1")]
                pub transfers: ::prost::alloc::vec::Vec<Transfer>,
            }

            #[derive(Clone, PartialEq, ::prost::Message)]
            pub struct Transfer {
                #[prost(string, tag = "1")]
                pub from: ::prost::alloc::string::String,
                #[prost(string, tag = "2")]
                pub to: ::prost::alloc::string::String,
                #[prost(uint64, tag = "3")]
                pub amount: u64,
            }
        }
    }
}
