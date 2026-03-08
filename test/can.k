devices [
  Device(
    config=DeviceConfig(
      name="PowerDistributionUnit"
      identifier="pdu"
      id=8
      type="STM32F7"
      include=["MinMaxAvg.hpp", "VNTypes.hpp", "ResetCause.hpp"]
      readonly=true
      update_frequency=100
      per_dos_uart="921600|PA10|PA9"
      frequency="Default:10"
      can_configuration="500000|PB12|PB13|Default"
      using="devices"
    )
    variables=[100]Variable
  )
]

true _
Bool true

Device (
  config DeviceConfig
  variables []Variable
)

DeviceConfig (
  name              String
  identifier        String
  id                I32
  type              String
  include           []String
  readonly          Bool
  update_frequency  I32
  per_dos_uart      String
  frequency         String
  can_configuration String
  using             String
)

for_some_reason_deviceconfig_is_becoming_an_empty_struct _
try_removing_it_from_config_on_device_struct _

Variable _

CanType (
  Bool CanType

  UInt8  CanType
  UInt16 CanType
  UInt32 CanType
  UInt64 CanType

  Int8  CanType
  Int16 CanType
  Int32 CanType
  Int64 CanType

  Float  CanType
  Double CanType
)
