#import "AppLifecycleHost.h"

#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

extern "C" void sdb_app_sleep_changed(bool sleeping);
extern "C" void sdb_app_screen_parameters_changed(void);

@interface SDBAppLifecycleHost : NSObject

@property(nonatomic, strong) id sleepObserver;
@property(nonatomic, strong) id wakeObserver;
@property(nonatomic, strong) id screenObserver;
@property(nonatomic, strong) id<NSObject> arcadeActivity;

@end

@implementation SDBAppLifecycleHost

+ (instancetype)sharedHost {
  static SDBAppLifecycleHost *host;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    host = [[SDBAppLifecycleHost alloc] init];
  });
  return host;
}

- (void)install {
  if (self.sleepObserver != nil || self.wakeObserver != nil ||
      self.screenObserver != nil) {
    return;
  }
  NSNotificationCenter *notifications = NSWorkspace.sharedWorkspace.notificationCenter;
  self.sleepObserver =
      [notifications addObserverForName:NSWorkspaceWillSleepNotification
                                 object:nil
                                  queue:NSOperationQueue.mainQueue
                             usingBlock:^(NSNotification *notification) {
                               (void)notification;
                               sdb_app_sleep_changed(true);
                             }];
  self.wakeObserver =
      [notifications addObserverForName:NSWorkspaceDidWakeNotification
                                 object:nil
                                  queue:NSOperationQueue.mainQueue
                             usingBlock:^(NSNotification *notification) {
                               (void)notification;
                               sdb_app_sleep_changed(false);
                             }];
  self.screenObserver =
      [NSNotificationCenter.defaultCenter
          addObserverForName:NSApplicationDidChangeScreenParametersNotification
                      object:nil
                       queue:NSOperationQueue.mainQueue
                  usingBlock:^(NSNotification *notification) {
                    (void)notification;
                    sdb_app_screen_parameters_changed();
                  }];
}

- (void)stop {
  [self setArcadeSessionActive:NO];
  NSNotificationCenter *notifications = NSWorkspace.sharedWorkspace.notificationCenter;
  if (self.sleepObserver != nil) {
    [notifications removeObserver:self.sleepObserver];
    self.sleepObserver = nil;
  }
  if (self.wakeObserver != nil) {
    [notifications removeObserver:self.wakeObserver];
    self.wakeObserver = nil;
  }
  if (self.screenObserver != nil) {
    [NSNotificationCenter.defaultCenter removeObserver:self.screenObserver];
    self.screenObserver = nil;
  }
}

- (void)setArcadeSessionActive:(BOOL)active {
  if (active && self.arcadeActivity == nil) {
    self.arcadeActivity = [NSProcessInfo.processInfo
        beginActivityWithOptions:(NSActivityIdleDisplaySleepDisabled |
                                  NSActivityIdleSystemSleepDisabled)
                        reason:@"Smart Dartboard arcade session"];
  } else if (!active && self.arcadeActivity != nil) {
    [NSProcessInfo.processInfo endActivity:self.arcadeActivity];
    self.arcadeActivity = nil;
  }
}

@end

void sdb_install_app_lifecycle_host(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [SDBAppLifecycleHost.sharedHost install];
  });
}

void sdb_stop_app_lifecycle_host(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [SDBAppLifecycleHost.sharedHost stop];
  });
}

void sdb_set_arcade_session_active(bool active) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [SDBAppLifecycleHost.sharedHost setArcadeSessionActive:active];
  });
}
