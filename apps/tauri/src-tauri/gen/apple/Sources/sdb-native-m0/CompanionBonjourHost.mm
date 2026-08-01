#import "CompanionBonjourHost.h"

#import <Foundation/Foundation.h>
#import <dns_sd.h>

#include <arpa/inet.h>

static DNSServiceRef SDBCompanionService = nullptr;
static dispatch_queue_t SDBCompanionBonjourQueue;

static dispatch_queue_t SDBBonjourQueue(void) {
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    SDBCompanionBonjourQueue = dispatch_queue_create(
        "de.gammaproduction.smart-dartboard.companion-bonjour",
        DISPATCH_QUEUE_SERIAL);
  });
  return SDBCompanionBonjourQueue;
}

static void SDBStopBonjourOnQueue(void) {
  if (SDBCompanionService != nullptr) {
    DNSServiceRefDeallocate(SDBCompanionService);
    SDBCompanionService = nullptr;
  }
}

static void DNSSD_API SDBRegistrationReply(
    DNSServiceRef, DNSServiceFlags, DNSServiceErrorType errorCode,
    const char *, const char *, const char *, void *) {
  if (errorCode != kDNSServiceErr_NoError) {
    NSLog(@"Smart Dartboard Bonjour registration failed: %d", errorCode);
  }
}

int32_t sdb_companion_bonjour_start(uint16_t port, const char *hostIDBytes,
                                    uint16_t protocolVersion) {
  if (port == 0 || hostIDBytes == nullptr) {
    return kDNSServiceErr_BadParam;
  }
  NSString *hostID = [NSString stringWithUTF8String:hostIDBytes];
  if (hostID == nil || hostID.length == 0 || hostID.length > 128) {
    return kDNSServiceErr_BadParam;
  }

  __block DNSServiceErrorType result = kDNSServiceErr_Unknown;
  dispatch_sync(SDBBonjourQueue(), ^{
    SDBStopBonjourOnQueue();

    TXTRecordRef txt;
    TXTRecordCreate(&txt, 0, nullptr);
    NSData *hostData = [hostID dataUsingEncoding:NSUTF8StringEncoding];
    NSString *version = [NSString stringWithFormat:@"%u", protocolVersion];
    NSData *versionData = [version dataUsingEncoding:NSUTF8StringEncoding];
    const uint8_t tls = '1';
    if (hostData.length > UINT8_MAX || versionData.length > UINT8_MAX ||
        TXTRecordSetValue(&txt, "id", static_cast<uint8_t>(hostData.length),
                          hostData.bytes) != kDNSServiceErr_NoError ||
        TXTRecordSetValue(&txt, "v", static_cast<uint8_t>(versionData.length),
                          versionData.bytes) != kDNSServiceErr_NoError ||
        TXTRecordSetValue(&txt, "tls", 1, &tls) != kDNSServiceErr_NoError) {
      TXTRecordDeallocate(&txt);
      result = kDNSServiceErr_BadParam;
      return;
    }

    NSString *shortID = [hostID substringToIndex:MIN((NSUInteger)8, hostID.length)];
    NSString *serviceName = [NSString stringWithFormat:@"Smart Dartboard %@", shortID];
    result = DNSServiceRegister(
        &SDBCompanionService, 0, 0, serviceName.UTF8String,
        "_sdb-darts._tcp", nullptr, nullptr, htons(port),
        TXTRecordGetLength(&txt), TXTRecordGetBytesPtr(&txt),
        SDBRegistrationReply, nullptr);
    TXTRecordDeallocate(&txt);
    if (result == kDNSServiceErr_NoError) {
      result = DNSServiceSetDispatchQueue(SDBCompanionService,
                                          SDBBonjourQueue());
    }
    if (result != kDNSServiceErr_NoError) {
      SDBStopBonjourOnQueue();
    }
  });
  return result;
}

void sdb_companion_bonjour_stop(void) {
  dispatch_sync(SDBBonjourQueue(), ^{
    SDBStopBonjourOnQueue();
  });
}
