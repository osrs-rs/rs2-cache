mod common;

mod osrs {
    use super::common;

    #[test]
    fn load_sample_varbit() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let varbit_loader = common::osrs::load_varbits(&cache)?;

        let chatbox_varbit = varbit_loader.load(8119).unwrap();

        assert_eq!(1737, chatbox_varbit.varp_id);
        assert_eq!(31, chatbox_varbit.least_significant_bit);
        assert_eq!(31, chatbox_varbit.most_significant_bit);

        Ok(())
    }
}
