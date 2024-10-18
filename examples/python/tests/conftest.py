from pathlib import Path

from example.baseline import FuelFigures, PerformanceFigures, Acceleration, BooleanType, Booster, BoostType, Engine, Model, Car, \
    OptionalExtras

STATIC_PATH = Path(__file__).parent / "static"


def load_file(path: str) -> bytes:
    return (STATIC_PATH / path).read_bytes()


def make_car() -> Car:
    optional_extras = OptionalExtras()
    optional_extras.sun_roof = False
    optional_extras.sports_pack = True
    optional_extras.cruise_control = True

    return Car(
        serial_number=1234,
        model_year=2013,
        available=BooleanType.T,
        code=Model.A,
        some_numbers=[1, 2, 3, 4],
        vehicle_code="abcdef",
        extras=optional_extras,
        engine=Engine(
            capacity=2000,
            num_cylinders=4,
            manufacturer_code="123",
            efficiency=35,
            booster_enabled=BooleanType.T,
            booster=Booster(
                boost_type=BoostType.NITROUS,
                horse_power=200,
            ),
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
        activation_code="abcdef",
    )
