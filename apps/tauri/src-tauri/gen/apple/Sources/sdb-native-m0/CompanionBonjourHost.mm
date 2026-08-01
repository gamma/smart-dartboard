#import "CompanionBonjourHost.h"

#import <Foundation/Foundation.h>
#import <dns_sd.h>

#include <arpa/inet.h>
#include <stdlib.h>
#include <string.h>

static DNSServiceRef SDBCompanionService = nullptr;
static DNSServiceRef SDBCompanionBrowser = nullptr;
static dispatch_queue_t SDBCompanionBonjourQueue;
static NSMutableDictionary<NSString *, NSDictionary *> *SDBDiscoveredHosts;

@interface SDBResolveRequest : NSObject

@property(nonatomic, assign) DNSServiceRef service;
@property(nonatomic, copy) NSString *key;

@end


@implementation SDBResolveRequest
@end

static NSMutableDictionary<NSString *, SDBResolveRequest *> *SDBResolveRequests;

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

static void SDBStopBrowserOnQueue(void) {
  if (SDBCompanionBrowser != nullptr) {
    DNSServiceRefDeallocate(SDBCompanionBrowser);
    SDBCompanionBrowser = nullptr;
  }
  for (SDBResolveRequest *request in SDBResolveRequests.allValues) {
    if (request.service != nullptr) {
      DNSServiceRefDeallocate(request.service);
      request.service = nullptr;
    }
  }
  [SDBResolveRequests removeAllObjects];
  [SDBDiscoveredHosts removeAllObjects];
}

static NSString *SDBServiceKey(uint32_t interfaceIndex, const char *name,
                               const char *domain) {
  NSString *serviceName = [NSString stringWithUTF8String:name ?: ""];
  NSString *serviceDomain = [NSString stringWithUTF8String:domain ?: ""];
  if (serviceName == nil || serviceDomain == nil) {
    return nil;
  }
  return [NSString stringWithFormat:@"%u|%@|%@", interfaceIndex, serviceName,
                                    serviceDomain];
}

static NSString *SDBTXTString(uint16_t txtLength, const unsigned char *txtRecord,
                              const char *key) {
  uint8_t valueLength = 0;
  const void *value =
      TXTRecordGetValuePtr(txtLength, txtRecord, key, &valueLength);
  if (value == nullptr || valueLength == 0) {
    return nil;
  }
  return [[NSString alloc] initWithBytes:value
                                  length:valueLength
                                encoding:NSUTF8StringEncoding];
}

static void DNSSD_API SDBResolveReply(
    DNSServiceRef, DNSServiceFlags, uint32_t, DNSServiceErrorType errorCode,
    const char *, const char *hostTarget, uint16_t port, uint16_t txtLength,
    const unsigned char *txtRecord, void *context) {
  SDBResolveRequest *request = (__bridge SDBResolveRequest *)context;
  if (request == nil) {
    return;
  }
  if (request.service != nullptr) {
    DNSServiceRefDeallocate(request.service);
    request.service = nullptr;
  }
  [SDBResolveRequests removeObjectForKey:request.key];
  if (errorCode != kDNSServiceErr_NoError || hostTarget == nullptr) {
    NSLog(@"Smart Dartboard Bonjour resolve failed: %d", errorCode);
    return;
  }

  NSString *hostID = SDBTXTString(txtLength, txtRecord, "id");
  NSString *versionText = SDBTXTString(txtLength, txtRecord, "v");
  NSString *tlsText = SDBTXTString(txtLength, txtRecord, "tls");
  NSString *hostName = [NSString stringWithUTF8String:hostTarget];
  if ([hostName hasSuffix:@"."]) {
    hostName = [hostName substringToIndex:hostName.length - 1];
  }
  NSInteger version = versionText.integerValue;
  uint16_t hostPort = ntohs(port);
  if (hostID.length == 0 || hostID.length > 128 || hostName.length == 0 ||
      hostName.length > 253 || version < 1 || version > UINT16_MAX ||
      ![tlsText isEqualToString:@"1"] || hostPort == 0) {
    return;
  }
  NSArray<NSString *> *parts = [request.key componentsSeparatedByString:@"|"];
  NSString *serviceName = parts.count > 1 ? parts[1] : @"Smart Dartboard";
  SDBDiscoveredHosts[request.key] = @{
    @"service_name" : serviceName,
    @"host_name" : hostName,
    @"port" : @(hostPort),
    @"host_id" : hostID,
    @"protocol_version" : @(version),
    @"tls" : @YES,
  };
}

static void DNSSD_API SDBBrowseReply(
    DNSServiceRef, DNSServiceFlags flags, uint32_t interfaceIndex,
    DNSServiceErrorType errorCode, const char *serviceName,
    const char *regtype, const char *replyDomain, void *) {
  if (errorCode != kDNSServiceErr_NoError) {
    NSLog(@"Smart Dartboard Bonjour browse failed: %d", errorCode);
    return;
  }
  NSString *key = SDBServiceKey(interfaceIndex, serviceName, replyDomain);
  if (key == nil) {
    return;
  }
  if ((flags & kDNSServiceFlagsAdd) == 0) {
    SDBResolveRequest *request = SDBResolveRequests[key];
    if (request.service != nullptr) {
      DNSServiceRefDeallocate(request.service);
      request.service = nullptr;
    }
    [SDBResolveRequests removeObjectForKey:key];
    [SDBDiscoveredHosts removeObjectForKey:key];
    return;
  }

  SDBResolveRequest *previous = SDBResolveRequests[key];
  if (previous.service != nullptr) {
    DNSServiceRefDeallocate(previous.service);
  }
  SDBResolveRequest *request = [[SDBResolveRequest alloc] init];
  request.key = key;
  DNSServiceRef resolver = nullptr;
  DNSServiceErrorType result = DNSServiceResolve(
      &resolver, 0, interfaceIndex, serviceName, regtype, replyDomain,
      SDBResolveReply, (__bridge void *)request);
  if (result != kDNSServiceErr_NoError) {
    NSLog(@"Smart Dartboard Bonjour resolve start failed: %d", result);
    return;
  }
  request.service = resolver;
  SDBResolveRequests[key] = request;
  result = DNSServiceSetDispatchQueue(request.service, SDBBonjourQueue());
  if (result != kDNSServiceErr_NoError) {
    DNSServiceRefDeallocate(request.service);
    request.service = nullptr;
    [SDBResolveRequests removeObjectForKey:key];
    NSLog(@"Smart Dartboard Bonjour resolve queue failed: %d", result);
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

int32_t sdb_companion_bonjour_browser_start(void) {
  __block DNSServiceErrorType result = kDNSServiceErr_Unknown;
  dispatch_sync(SDBBonjourQueue(), ^{
    SDBStopBrowserOnQueue();
    SDBDiscoveredHosts = [[NSMutableDictionary alloc] init];
    SDBResolveRequests = [[NSMutableDictionary alloc] init];
    result = DNSServiceBrowse(&SDBCompanionBrowser, 0, 0, "_sdb-darts._tcp",
                              nullptr, SDBBrowseReply, nullptr);
    if (result == kDNSServiceErr_NoError) {
      result = DNSServiceSetDispatchQueue(SDBCompanionBrowser,
                                          SDBBonjourQueue());
    }
    if (result != kDNSServiceErr_NoError) {
      SDBStopBrowserOnQueue();
    }
  });
  return result;
}

void sdb_companion_bonjour_browser_stop(void) {
  dispatch_sync(SDBBonjourQueue(), ^{
    SDBStopBrowserOnQueue();
  });
}

int32_t sdb_companion_bonjour_browser_snapshot(uint8_t **bytes,
                                               size_t *length) {
  if (bytes == nullptr || length == nullptr) {
    return -1;
  }
  *bytes = nullptr;
  *length = 0;
  __block NSData *json = nil;
  dispatch_sync(SDBBonjourQueue(), ^{
    NSArray<NSDictionary *> *hosts = [SDBDiscoveredHosts.allValues
        sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *left,
                                                        NSDictionary *right) {
          NSComparisonResult name = [left[@"service_name"]
              compare:right[@"service_name"]];
          if (name != NSOrderedSame) {
            return name;
          }
          return [left[@"host_id"] compare:right[@"host_id"]];
        }];
    json = [NSJSONSerialization dataWithJSONObject:hosts ?: @[]
                                           options:0
                                             error:nil];
  });
  if (json == nil || json.length == 0) {
    return -1;
  }
  uint8_t *copy = static_cast<uint8_t *>(malloc(json.length));
  if (copy == nullptr) {
    return -1;
  }
  memcpy(copy, json.bytes, json.length);
  *bytes = copy;
  *length = json.length;
  return 0;
}

void sdb_companion_bonjour_browser_snapshot_free(uint8_t *bytes,
                                                 size_t) {
  free(bytes);
}
