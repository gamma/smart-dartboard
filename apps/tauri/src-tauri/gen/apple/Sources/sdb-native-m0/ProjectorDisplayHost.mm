#import "ProjectorDisplayHost.h"

#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>

#include <stdint.h>

extern "C" void sdb_external_display_changed(uint32_t display_count);

@interface SDBProjectorDisplayHost : NSObject <WKNavigationDelegate>

@property(nonatomic, strong) NSMutableDictionary<NSValue *, UIWindow *> *windows;
@property(nonatomic, copy) NSString *latestStateJSON;

@end

@implementation SDBProjectorDisplayHost

+ (instancetype)sharedHost {
  static SDBProjectorDisplayHost *host;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    host = [[SDBProjectorDisplayHost alloc] init];
  });
  return host;
}

- (instancetype)init {
  self = [super init];
  if (self) {
    _windows = [NSMutableDictionary dictionary];
    _latestStateJSON = @"{\"runtime_instance_id\":\"native-m0\",\"revision\":0,\"counter\":0}";
  }
  return self;
}

- (void)install {
  NSLog(@"Smart Dartboard DisplayHost: installing");
  NSNotificationCenter *notifications = NSNotificationCenter.defaultCenter;
  [notifications addObserver:self
                    selector:@selector(screenDidConnect:)
                        name:UIScreenDidConnectNotification
                      object:nil];
  [notifications addObserver:self
                    selector:@selector(screenDidDisconnect:)
                        name:UIScreenDidDisconnectNotification
                      object:nil];

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  for (UIScreen *screen in UIScreen.screens) {
    if (screen != UIScreen.mainScreen) {
      [self attachScreen:screen];
    }
  }
#pragma clang diagnostic pop

  [self publishDisplayCount];
}

- (void)screenDidConnect:(NSNotification *)notification {
  UIScreen *screen = notification.object;
  if (screen != nil) {
    [self attachScreen:screen];
  }
}

- (void)screenDidDisconnect:(NSNotification *)notification {
  UIScreen *screen = notification.object;
  if (screen == nil) {
    return;
  }

  NSValue *key = [NSValue valueWithNonretainedObject:screen];
  UIWindow *window = self.windows[key];
  window.hidden = YES;
  [self.windows removeObjectForKey:key];
  [self publishDisplayCount];
}

- (void)attachScreen:(UIScreen *)screen {
  NSValue *key = [NSValue valueWithNonretainedObject:screen];
  if (self.windows[key] != nil) {
    return;
  }

  WKWebViewConfiguration *configuration = [[WKWebViewConfiguration alloc] init];
  WKWebView *webView = [[WKWebView alloc] initWithFrame:CGRectZero
                                         configuration:configuration];
  webView.navigationDelegate = self;
  webView.opaque = NO;
  webView.backgroundColor = UIColor.clearColor;

  UIViewController *controller = [[UIViewController alloc] init];
  controller.view = webView;

  UIWindow *window = [[UIWindow alloc] initWithFrame:screen.bounds];
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  window.screen = screen;
  screen.overscanCompensation = UIScreenOverscanCompensationNone;
#pragma clang diagnostic pop
  window.rootViewController = controller;
  window.hidden = NO;
  self.windows[key] = window;
  NSLog(@"Smart Dartboard DisplayHost: attached %@ (%0.fx%0.f)", screen,
        screen.bounds.size.width, screen.bounds.size.height);

  [webView loadHTMLString:[self projectorHTML] baseURL:nil];
  [self publishDisplayCount];
}

- (NSString *)projectorHTML {
  return @"<!doctype html><html lang='de'><head>"
          "<meta charset='utf-8'>"
          "<meta name='viewport' content='width=device-width,initial-scale=1'>"
          "<style>"
          ":root{color-scheme:dark;font-family:-apple-system,BlinkMacSystemFont,sans-serif}"
          "*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;"
          "overflow:hidden;background:radial-gradient(circle at 70% 25%,#123649 0,transparent 38%),#05070b;color:#f5f8ff}"
          "main{text-align:center}.role{color:#28e7ff;font-size:2.2vw;font-weight:900;letter-spacing:.24em}"
          "h1{font-size:8vw;line-height:.9;letter-spacing:-.06em;margin:3vh 0}.counter{font-size:22vw;"
          "font-weight:950;color:#28e7ff;line-height:.9}.status{color:#a8b7ca;font-size:2vw;margin-top:4vh}"
          "</style></head><body><main><div class='role'>PROJECTOR · AIRPLAY / HDMI</div>"
          "<h1>Eine Runtime.<br>Zwei Screens.</h1><div id='counter' class='counter'>0</div>"
          "<div id='status' class='status'>Runtime wird verbunden …</div></main>"
          "<script>window.sdbApplyState=function(s){document.getElementById('counter').textContent=String(s.counter??0);"
          "document.getElementById('status').textContent='Runtime '+s.runtime_instance_id+' · Revision '+s.revision;};"
          "</script></body></html>";
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
  (void)navigation;
  [self applyLatestStateToWebView:webView];
}

- (void)applyLatestStateToWebView:(WKWebView *)webView {
  NSString *script = [NSString stringWithFormat:@"window.sdbApplyState(%@);", self.latestStateJSON];
  [webView evaluateJavaScript:script completionHandler:nil];
}

- (void)updateStateJSON:(NSString *)stateJSON {
  self.latestStateJSON = stateJSON;
  for (UIWindow *window in self.windows.allValues) {
    WKWebView *webView = (WKWebView *)window.rootViewController.view;
    [self applyLatestStateToWebView:webView];
  }
}

- (void)publishDisplayCount {
  NSLog(@"Smart Dartboard DisplayHost: %lu external display(s)",
        (unsigned long)self.windows.count);
  sdb_external_display_changed((uint32_t)self.windows.count);
}

@end

void sdb_install_projector_display_host(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    [[SDBProjectorDisplayHost sharedHost] install];
  });
}

void sdb_projector_update(const char *state_json) {
  if (state_json == nullptr) {
    return;
  }
  NSString *json = [NSString stringWithUTF8String:state_json];
  dispatch_async(dispatch_get_main_queue(), ^{
    [[SDBProjectorDisplayHost sharedHost] updateStateJSON:json];
  });
}
