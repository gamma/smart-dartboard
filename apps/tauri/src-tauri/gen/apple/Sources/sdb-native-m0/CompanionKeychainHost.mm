#import "CompanionKeychainHost.h"

#import <Foundation/Foundation.h>
#import <Security/Security.h>

#include <stdlib.h>
#include <string.h>

static NSString *const SDBKeychainService = @"de.gammaproduction.smart-dartboard.companion";

static void SDBSecureZero(uint8_t *bytes, size_t length) {
  volatile uint8_t *cursor = bytes;
  while (length > 0) {
    *cursor = 0;
    ++cursor;
    --length;
  }
}

static NSDictionary *SDBKeychainQuery(NSString *account) {
  return @{
    (__bridge id)kSecClass : (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrService : SDBKeychainService,
    (__bridge id)kSecAttrAccount : account,
  };
}

int32_t sdb_keychain_load(const char *accountBytes, uint8_t **bytes, size_t *length) {
  if (accountBytes == nullptr || bytes == nullptr || length == nullptr) {
    return -1;
  }
  *bytes = nullptr;
  *length = 0;
  NSString *account = [NSString stringWithUTF8String:accountBytes];
  if (account == nil || account.length == 0) {
    return -1;
  }
  NSMutableDictionary *query = [SDBKeychainQuery(account) mutableCopy];
  query[(__bridge id)kSecReturnData] = @YES;
  query[(__bridge id)kSecMatchLimit] = (__bridge id)kSecMatchLimitOne;
  CFTypeRef result = nullptr;
  OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);
  if (status == errSecItemNotFound) {
    return 0;
  }
  if (status != errSecSuccess || result == nullptr) {
    if (result != nullptr) {
      CFRelease(result);
    }
    return -1;
  }
  NSData *data = CFBridgingRelease(result);
  if (data.length == 0) {
    return -1;
  }
  uint8_t *copy = static_cast<uint8_t *>(malloc(data.length));
  if (copy == nullptr) {
    return -1;
  }
  memcpy(copy, data.bytes, data.length);
  *bytes = copy;
  *length = data.length;
  return 1;
}

bool sdb_keychain_save(const char *accountBytes, const uint8_t *bytes, size_t length) {
  if (accountBytes == nullptr || bytes == nullptr || length == 0) {
    return false;
  }
  NSString *account = [NSString stringWithUTF8String:accountBytes];
  if (account == nil || account.length == 0) {
    return false;
  }
  NSData *data = [NSData dataWithBytes:bytes length:length];
  NSDictionary *query = SDBKeychainQuery(account);
  NSDictionary *update = @{(__bridge id)kSecValueData : data};
  OSStatus status = SecItemUpdate((__bridge CFDictionaryRef)query,
                                  (__bridge CFDictionaryRef)update);
  if (status == errSecSuccess) {
    return true;
  }
  if (status != errSecItemNotFound) {
    return false;
  }
  NSMutableDictionary *insert = [query mutableCopy];
  insert[(__bridge id)kSecValueData] = data;
  insert[(__bridge id)kSecAttrAccessible] =
      (__bridge id)kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly;
  return SecItemAdd((__bridge CFDictionaryRef)insert, nullptr) == errSecSuccess;
}

void sdb_keychain_free(uint8_t *bytes, size_t length) {
  if (bytes != nullptr && length > 0) {
    SDBSecureZero(bytes, length);
  }
  free(bytes);
}
