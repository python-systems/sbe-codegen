use example::baseline::decoder::ReadBuf;
use example::baseline::encoder::WriteBuf;
use example::baseline::enums::{BooleanType, BoostType, Model};
use example::baseline::messages::{CarDecoder, CarEncoder};
use example::baseline::sets::OptionalExtras;
use rstest::rstest;

#[rstest]
fn test_car_encode() {
    let original = include_bytes!("static/example_schema_car.sbe");

    let mut buffer = [0u8; 1024];
    let write_buf = WriteBuf::new(&mut buffer);
    let mut car = CarEncoder::try_from(write_buf).unwrap();

    car.serial_number(1234).unwrap();
    car.model_year(2013).unwrap();
    car.available(BooleanType::T).unwrap();
    car.code(Model::A).unwrap();
    car.some_numbers(&[1, 2, 3, 4]).unwrap();
    car.vehicle_code("abcdef").unwrap();

    let mut extras = OptionalExtras::default();
    extras.set_sun_roof(false);
    extras.set_sports_pack(true);
    extras.set_cruise_control(true);
    car.extras(extras).unwrap();

    car.engine_encoder(|engine| {
        engine.capacity(2000)?;
        engine.num_cylinders(4)?;
        engine.manufacturer_code("123")?;
        engine.efficiency(35)?;
        engine.booster_enabled(BooleanType::T)?;
        engine.booster_encoder(|booster| {
            booster.boost_type(BoostType::Nitrous)?;
            booster.horse_power(200)
        })
    })
    .unwrap();

    car.fuel_figures_encoder(|fuel_figures| {
        fuel_figures.speed(30)?;
        fuel_figures.mpg(35.9)?;
        fuel_figures.usage_description_encoder(|usage_description| {
            usage_description.put_slice_at(0, b"Urban Cycle")
        })?;
        fuel_figures.advance()?;

        fuel_figures.speed(55)?;
        fuel_figures.mpg(49.0)?;
        fuel_figures.usage_description_encoder(|usage_description| {
            usage_description.put_slice_at(0, b"Combined Cycle")
        })?;
        fuel_figures.advance()?;

        fuel_figures.speed(75)?;
        fuel_figures.mpg(40.0)?;
        fuel_figures.usage_description_encoder(|usage_description| {
            usage_description.put_slice_at(0, b"Highway Cycle")
        })?;
        fuel_figures.advance()
    })
    .unwrap();

    car.performance_figures_encoder(|performance_figures| {
        performance_figures.octane_rating(95)?;
        performance_figures.acceleration_encoder(|acceleration| {
            acceleration.mph(30)?;
            acceleration.seconds(4.0)?;
            acceleration.advance()?;

            acceleration.mph(60)?;
            acceleration.seconds(7.5)?;
            acceleration.advance()?;

            acceleration.mph(100)?;
            acceleration.seconds(12.2)?;
            acceleration.advance()
        })?;
        performance_figures.advance()?;

        performance_figures.octane_rating(99)?;
        performance_figures.acceleration_encoder(|acceleration| {
            acceleration.mph(30)?;
            acceleration.seconds(3.8)?;
            acceleration.advance()?;

            acceleration.mph(60)?;
            acceleration.seconds(7.1)?;
            acceleration.advance()?;

            acceleration.mph(100)?;
            acceleration.seconds(11.8)?;
            acceleration.advance()
        })?;
        performance_figures.advance()
    })
    .unwrap();

    car.manufacturer_encoder(|manufacturer| manufacturer.put_slice_at(0, b"Honda"))
        .unwrap();

    car.model_encoder(|model| model.put_slice_at(0, b"Civic VTi"))
        .unwrap();

    car.activation_code_encoder(|activation_code| activation_code.put_slice_at(0, b"abcdef"))
        .unwrap();

    let encoded_size = car.size().unwrap();

    assert_eq!(&buffer[..encoded_size], original)
}

#[rstest]
fn test_car_decode() {
    let original = include_bytes!("static/example_schema_car.sbe");

    let read_buf = ReadBuf::new(original);
    let mut car = CarDecoder::try_from(read_buf).unwrap();

    assert_eq!(car.serial_number().unwrap(), 1234);
    assert_eq!(car.model_year().unwrap(), 2013);
    assert_eq!(car.available().unwrap(), BooleanType::T);
    assert_eq!(car.code().unwrap(), Model::A);
    assert_eq!(car.some_numbers().unwrap(), [1, 2, 3, 4]);
    assert_eq!(car.vehicle_code().unwrap(), "abcdef");

    let extras = car.extras().unwrap();
    assert!(!extras.get_sun_roof());
    assert!(extras.get_sports_pack());
    assert!(extras.get_cruise_control());

    car.engine_decoder(|engine| {
        assert_eq!(engine.capacity().unwrap(), 2000);
        assert_eq!(engine.num_cylinders().unwrap(), 4);
        assert_eq!(engine.manufacturer_code().unwrap(), "123");
        assert_eq!(engine.efficiency().unwrap(), 35);
        assert_eq!(engine.booster_enabled().unwrap(), BooleanType::T);

        engine.booster_decoder(|booster| {
            assert_eq!(booster.boost_type().unwrap(), BoostType::Nitrous);
            assert_eq!(booster.horse_power().unwrap(), 200);
            Ok(())
        })
    })
    .unwrap();

    car.fuel_figures_decoder(|fuel_figures| {
        assert_eq!(fuel_figures.speed().unwrap(), 30);
        assert_eq!(fuel_figures.mpg().unwrap(), 35.9);
        assert_eq!(
            fuel_figures
                .usage_description_decoder(|usage_description| {
                    Ok(String::from_utf8(
                        usage_description
                            .get_slice_at(0, usage_description.length())?
                            .to_vec(),
                    )?)
                })
                .unwrap(),
            "Urban Cycle"
        );
        fuel_figures.advance().unwrap();

        assert_eq!(fuel_figures.speed().unwrap(), 55);
        assert_eq!(fuel_figures.mpg().unwrap(), 49.0);
        assert_eq!(
            fuel_figures
                .usage_description_decoder(|usage_description| {
                    Ok(String::from_utf8(
                        usage_description
                            .get_slice_at(0, usage_description.length())?
                            .to_vec(),
                    )?)
                })
                .unwrap(),
            "Combined Cycle"
        );
        fuel_figures.advance().unwrap();

        assert_eq!(fuel_figures.speed().unwrap(), 75);
        assert_eq!(fuel_figures.mpg().unwrap(), 40.0);
        assert_eq!(
            fuel_figures
                .usage_description_decoder(|usage_description| {
                    Ok(String::from_utf8(
                        usage_description
                            .get_slice_at(0, usage_description.length())?
                            .to_vec(),
                    )?)
                })
                .unwrap(),
            "Highway Cycle"
        );
        fuel_figures.advance()
    })
    .unwrap();

    car.performance_figures_decoder(|performance_figures| {
        assert_eq!(performance_figures.octane_rating().unwrap(), 95);
        performance_figures
            .acceleration_decoder(|acceleration| {
                assert_eq!(acceleration.mph().unwrap(), 30);
                assert_eq!(acceleration.seconds().unwrap(), 4.0);
                acceleration.advance().unwrap();

                assert_eq!(acceleration.mph().unwrap(), 60);
                assert_eq!(acceleration.seconds().unwrap(), 7.5);
                acceleration.advance().unwrap();

                assert_eq!(acceleration.mph().unwrap(), 100);
                assert_eq!(acceleration.seconds().unwrap(), 12.2);
                acceleration.advance()
            })
            .unwrap();
        performance_figures.advance().unwrap();

        assert_eq!(performance_figures.octane_rating().unwrap(), 99);
        performance_figures
            .acceleration_decoder(|acceleration| {
                assert_eq!(acceleration.mph().unwrap(), 30);
                assert_eq!(acceleration.seconds().unwrap(), 3.8);
                acceleration.advance().unwrap();

                assert_eq!(acceleration.mph().unwrap(), 60);
                assert_eq!(acceleration.seconds().unwrap(), 7.1);
                acceleration.advance().unwrap();

                assert_eq!(acceleration.mph().unwrap(), 100);
                assert_eq!(acceleration.seconds().unwrap(), 11.8);
                acceleration.advance()
            })
            .unwrap();
        performance_figures.advance()
    })
    .unwrap();
}
