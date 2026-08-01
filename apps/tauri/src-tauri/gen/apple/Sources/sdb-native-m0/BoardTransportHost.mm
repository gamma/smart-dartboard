#import "BoardTransportHost.h"

#import <CoreBluetooth/CoreBluetooth.h>
#import <Foundation/Foundation.h>

#include <stddef.h>
#include <stdint.h>

extern "C" void sdb_board_status_changed(uint32_t phase, int32_t failure,
                                           const char *detail,
                                           const char *connection_id);
extern "C" void sdb_board_notification(const uint8_t *bytes, size_t length,
                                         const char *connection_id);

static NSString *const SDBRestoreIdentifier = @"de.gammaproduction.smart-dartboard.central";
static NSString *const SDBPeripheralIdentifierKey = @"sdb.board.peripheral-identifier";
static NSString *const SDBDeviceName = @"SDB-BT";

typedef NS_ENUM(uint32_t, SDBBoardPhase) {
  SDBBoardPhaseUnavailable = 0,
  SDBBoardPhasePermissionRequired = 1,
  SDBBoardPhaseBluetoothOff = 2,
  SDBBoardPhaseScanning = 3,
  SDBBoardPhaseConnecting = 4,
  SDBBoardPhaseDiscovering = 5,
  SDBBoardPhaseSubscribing = 6,
  SDBBoardPhaseReady = 7,
  SDBBoardPhaseReconnecting = 8,
  SDBBoardPhaseError = 9,
};

typedef NS_ENUM(int32_t, SDBBoardFailure) {
  SDBBoardFailureNone = 0,
  SDBBoardFailureAdapterUnavailable = 1,
  SDBBoardFailurePermissionDenied = 2,
  SDBBoardFailureBluetoothPoweredOff = 3,
  SDBBoardFailureDeviceNotFound = 4,
  SDBBoardFailureConnectionFailed = 5,
  SDBBoardFailureServiceMissing = 6,
  SDBBoardFailureCharacteristicMissing = 7,
  SDBBoardFailureSubscriptionFailed = 8,
  SDBBoardFailureTransportError = 11,
};

@interface SDBBoardTransportHost : NSObject <CBCentralManagerDelegate, CBPeripheralDelegate>

@property(nonatomic, strong) CBCentralManager *central;
@property(nonatomic, strong) CBPeripheral *peripheral;
@property(nonatomic, strong) CBUUID *serviceUUID;
@property(nonatomic, strong) CBUUID *notifyUUID;
@property(nonatomic, copy) NSString *connectionID;

@end

@implementation SDBBoardTransportHost

+ (instancetype)sharedHost {
  static SDBBoardTransportHost *host;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    host = [[SDBBoardTransportHost alloc] init];
  });
  return host;
}

- (instancetype)init {
  self = [super init];
  if (self) {
    _serviceUUID = [CBUUID UUIDWithString:@"FFF0"];
    _notifyUUID = [CBUUID UUIDWithString:@"FFF1"];
  }
  return self;
}

- (void)install {
  if (self.central != nil) {
    return;
  }
  NSDictionary *options = @{CBCentralManagerOptionRestoreIdentifierKey : SDBRestoreIdentifier};
  self.central = [[CBCentralManager alloc] initWithDelegate:self
                                                       queue:dispatch_get_main_queue()
                                                     options:options];
}

- (void)publishPhase:(SDBBoardPhase)phase
              failure:(SDBBoardFailure)failure
                detail:(NSString *)detail {
  sdb_board_status_changed(phase, failure, detail.UTF8String,
                           self.connectionID.UTF8String);
}

- (void)centralManagerDidUpdateState:(CBCentralManager *)central {
  switch (central.state) {
    case CBManagerStatePoweredOn:
      [self restoreKnownPeripheralOrScan];
      break;
    case CBManagerStateUnauthorized:
      [self publishPhase:SDBBoardPhasePermissionRequired
                 failure:SDBBoardFailurePermissionDenied
                   detail:@"Bluetooth permission is required"];
      break;
    case CBManagerStatePoweredOff:
      [self publishPhase:SDBBoardPhaseBluetoothOff
                 failure:SDBBoardFailureBluetoothPoweredOff
                   detail:@"Bluetooth is powered off"];
      break;
    case CBManagerStateUnsupported:
      [self publishPhase:SDBBoardPhaseUnavailable
                 failure:SDBBoardFailureAdapterUnavailable
                   detail:@"Bluetooth LE is not supported"];
      break;
    default:
      [self publishPhase:SDBBoardPhaseUnavailable
                 failure:SDBBoardFailureAdapterUnavailable
                   detail:@"Bluetooth adapter is not ready"];
      break;
  }
}

- (void)restoreKnownPeripheralOrScan {
  if (self.peripheral != nil) {
    if (self.peripheral.state == CBPeripheralStateConnected) {
      [self publishPhase:SDBBoardPhaseDiscovering failure:SDBBoardFailureNone detail:nil];
      [self.peripheral discoverServices:@[self.serviceUUID]];
    } else {
      [self connectPeripheral:self.peripheral];
    }
    return;
  }
  NSString *saved = [NSUserDefaults.standardUserDefaults stringForKey:SDBPeripheralIdentifierKey];
  NSUUID *identifier = saved == nil ? nil : [[NSUUID alloc] initWithUUIDString:saved];
  if (identifier != nil) {
    NSArray<CBPeripheral *> *known = [self.central retrievePeripheralsWithIdentifiers:@[identifier]];
    if (known.count > 0) {
      [self connectPeripheral:known.firstObject];
      return;
    }
  }
  [self scan];
}

- (void)scan {
  self.peripheral = nil;
  self.connectionID = nil;
  [self publishPhase:SDBBoardPhaseScanning failure:SDBBoardFailureNone detail:nil];
  [self.central scanForPeripheralsWithServices:nil
                                      options:@{CBCentralManagerScanOptionAllowDuplicatesKey : @NO}];
}

- (void)connectPeripheral:(CBPeripheral *)peripheral {
  [self.central stopScan];
  self.peripheral = peripheral;
  self.peripheral.delegate = self;
  self.connectionID = NSUUID.UUID.UUIDString;
  [self publishPhase:SDBBoardPhaseConnecting failure:SDBBoardFailureNone detail:nil];
  [self.central connectPeripheral:peripheral
                          options:@{CBConnectPeripheralOptionNotifyOnDisconnectionKey : @YES}];
}

- (void)centralManager:(CBCentralManager *)central
 didDiscoverPeripheral:(CBPeripheral *)peripheral
     advertisementData:(NSDictionary<NSString *, id> *)advertisementData
                  RSSI:(NSNumber *)RSSI {
  (void)central;
  (void)RSSI;
  NSString *localName = advertisementData[CBAdvertisementDataLocalNameKey];
  NSArray<CBUUID *> *services = advertisementData[CBAdvertisementDataServiceUUIDsKey];
  if ((peripheral.name != nil && [peripheral.name isEqualToString:SDBDeviceName]) ||
      (localName != nil && [localName isEqualToString:SDBDeviceName]) ||
      [services containsObject:self.serviceUUID]) {
    [NSUserDefaults.standardUserDefaults setObject:peripheral.identifier.UUIDString
                                            forKey:SDBPeripheralIdentifierKey];
    [self connectPeripheral:peripheral];
  }
}

- (void)centralManager:(CBCentralManager *)central
    didConnectPeripheral:(CBPeripheral *)peripheral {
  (void)central;
  [self publishPhase:SDBBoardPhaseDiscovering failure:SDBBoardFailureNone detail:nil];
  peripheral.delegate = self;
  [peripheral discoverServices:@[self.serviceUUID]];
}

- (void)centralManager:(CBCentralManager *)central
 didFailToConnectPeripheral:(CBPeripheral *)peripheral
                  error:(NSError *)error {
  (void)central;
  (void)peripheral;
  [self publishPhase:SDBBoardPhaseReconnecting
             failure:SDBBoardFailureConnectionFailed
               detail:error.localizedDescription];
  [self scan];
}

- (void)centralManager:(CBCentralManager *)central
 didDisconnectPeripheral:(CBPeripheral *)peripheral
                   timestamp:(CFAbsoluteTime)timestamp
        isReconnecting:(BOOL)isReconnecting
                    error:(NSError *)error API_AVAILABLE(ios(17.0)) {
  (void)central;
  (void)peripheral;
  (void)timestamp;
  (void)isReconnecting;
  [self publishPhase:SDBBoardPhaseReconnecting
             failure:SDBBoardFailureConnectionFailed
               detail:error.localizedDescription];
  [self scan];
}

- (void)centralManager:(CBCentralManager *)central
 didDisconnectPeripheral:(CBPeripheral *)peripheral
                  error:(NSError *)error {
  (void)central;
  (void)peripheral;
  [self publishPhase:SDBBoardPhaseReconnecting
             failure:SDBBoardFailureConnectionFailed
               detail:error.localizedDescription];
  [self scan];
}

- (void)centralManager:(CBCentralManager *)central
      willRestoreState:(NSDictionary<NSString *, id> *)dict {
  (void)central;
  NSArray<CBPeripheral *> *peripherals = dict[CBCentralManagerRestoredStatePeripheralsKey];
  CBPeripheral *restored = peripherals.firstObject;
  if (restored != nil) {
    self.peripheral = restored;
    restored.delegate = self;
    self.connectionID = NSUUID.UUID.UUIDString;
  }
}

- (void)peripheral:(CBPeripheral *)peripheral
 didDiscoverServices:(NSError *)error {
  if (error != nil) {
    [self publishPhase:SDBBoardPhaseError
               failure:SDBBoardFailureServiceMissing
                 detail:error.localizedDescription];
    [self.central cancelPeripheralConnection:peripheral];
    return;
  }
  for (CBService *service in peripheral.services) {
    if ([service.UUID isEqual:self.serviceUUID]) {
      [peripheral discoverCharacteristics:@[self.notifyUUID] forService:service];
      return;
    }
  }
  [self publishPhase:SDBBoardPhaseError
             failure:SDBBoardFailureServiceMissing
               detail:@"FFF0 service is missing"];
  [self.central cancelPeripheralConnection:peripheral];
}

- (void)peripheral:(CBPeripheral *)peripheral
 didDiscoverCharacteristicsForService:(CBService *)service
              error:(NSError *)error {
  if (error != nil) {
    [self publishPhase:SDBBoardPhaseError
               failure:SDBBoardFailureCharacteristicMissing
                 detail:error.localizedDescription];
    [self.central cancelPeripheralConnection:peripheral];
    return;
  }
  for (CBCharacteristic *characteristic in service.characteristics) {
    if ([characteristic.UUID isEqual:self.notifyUUID]) {
      [self publishPhase:SDBBoardPhaseSubscribing failure:SDBBoardFailureNone detail:nil];
      [peripheral setNotifyValue:YES forCharacteristic:characteristic];
      return;
    }
  }
  [self publishPhase:SDBBoardPhaseError
             failure:SDBBoardFailureCharacteristicMissing
               detail:@"FFF1 characteristic is missing"];
  [self.central cancelPeripheralConnection:peripheral];
}

- (void)peripheral:(CBPeripheral *)peripheral
 didUpdateNotificationStateForCharacteristic:(CBCharacteristic *)characteristic
              error:(NSError *)error {
  if (error != nil || !characteristic.isNotifying) {
    [self publishPhase:SDBBoardPhaseError
               failure:SDBBoardFailureSubscriptionFailed
                 detail:error.localizedDescription ?: @"FFF1 subscription failed"];
    [self.central cancelPeripheralConnection:peripheral];
    return;
  }
  [self publishPhase:SDBBoardPhaseReady failure:SDBBoardFailureNone detail:nil];
}

- (void)peripheral:(CBPeripheral *)peripheral
 didUpdateValueForCharacteristic:(CBCharacteristic *)characteristic
              error:(NSError *)error {
  (void)peripheral;
  if (error != nil) {
    [self publishPhase:SDBBoardPhaseError
               failure:SDBBoardFailureTransportError
                 detail:error.localizedDescription];
    return;
  }
  NSData *value = characteristic.value;
  if (value != nil && self.connectionID != nil) {
    sdb_board_notification(static_cast<const uint8_t *>(value.bytes), value.length,
                           self.connectionID.UTF8String);
  }
}

@end

void sdb_install_board_transport_host(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [[SDBBoardTransportHost sharedHost] install];
  });
}
