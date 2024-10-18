from conftest import load_file, make_car
from example.baseline import (
    Car,
    Model,
    BoostType,
    FuelFigures,
    PerformanceFigures,
    Acceleration,
    BooleanType,
    OptionalExtras,
    Engine,
    Booster
)

def test_subclass():
    # Might throw type error if pyo3 is not configured correctly
    class SubBooster(Booster):
        pass

    _instance = SubBooster(boost_type=BoostType.NITROUS, horse_power=200)


def test_car_encode():
    car = make_car()
    car_bytes = car.to_bytes(512)
    car_bytes_ref = load_file("example_schema_car.sbe")

    assert car_bytes == car_bytes_ref


def test_car_encode_buffer():
    car = make_car()
    buffer = bytearray(512)
    encoded_size = car.write_to_buffer(buffer)

    car_bytes_ref = load_file("example_schema_car.sbe")

    assert buffer[:encoded_size] == car_bytes_ref


def test_car_decode():
    car_bytes_ref = load_file("example_schema_car.sbe")

    car = Car.from_bytes(car_bytes_ref)

    extras = OptionalExtras()
    extras.sun_roof = False
    extras.sports_pack = True
    extras.cruise_control = True

    assert car == Car(
        serial_number=1234,
        model_year=2013,
        available=BooleanType.T,
        code=Model.A,
        some_numbers=[1, 2, 3, 4],
        vehicle_code="abcdef",
        extras=extras,
        engine=Engine(
            capacity=2000,
            num_cylinders=4,
            manufacturer_code="123",
            efficiency=35,
            booster_enabled=BooleanType.T,
            booster=Booster(
                boost_type=BoostType.NITROUS,
                horse_power=200
            )
        ),
        fuel_figures=[
            FuelFigures(speed=30, mpg=35.9, usage_description="Urban Cycle"),
            FuelFigures(speed=55, mpg=49.0, usage_description="Combined Cycle"),
            FuelFigures(speed=75, mpg=40.0, usage_description="Highway Cycle"),
        ],
        performance_figures=[
            PerformanceFigures(octane_rating=95, acceleration=[
                Acceleration(mph=30, seconds=4.0),
                Acceleration(mph=60, seconds=7.5),
                Acceleration(mph=100, seconds=12.2),
            ]),
            PerformanceFigures(octane_rating=99, acceleration=[
                Acceleration(mph=30, seconds=3.8),
                Acceleration(mph=60, seconds=7.1),
                Acceleration(mph=100, seconds=11.8),
            ])
        ],
        manufacturer="Honda",
        model="Civic VTi",
        activation_code="abcdef"
    )
