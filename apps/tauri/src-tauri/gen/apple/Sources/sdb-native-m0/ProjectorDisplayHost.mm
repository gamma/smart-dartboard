#import "ProjectorDisplayHost.h"

#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>

#include <stdint.h>

extern "C" void sdb_external_display_changed(uint32_t display_count);

typedef struct {
  uint8_t *data;
  size_t length;
  char *mime;
} SDBProjectorAsset;

extern "C" bool sdb_projector_asset(const char *path, SDBProjectorAsset *output);
extern "C" void sdb_projector_asset_free(SDBProjectorAsset asset);
extern "C" char *sdb_projector_command(const char *command_json);
extern "C" char *sdb_projector_effect_ack(const char *effect_id);
extern "C" void sdb_projector_string_free(char *value);

@interface SDBProjectorDisplayHost : NSObject <WKNavigationDelegate,
                                                WKURLSchemeHandler,
                                                WKScriptMessageHandler>

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
    _latestStateJSON = nil;
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
  [configuration setURLSchemeHandler:self forURLScheme:@"sdb-projector"];
  WKUserContentController *contentController = [[WKUserContentController alloc] init];
  [contentController addScriptMessageHandler:self name:@"sdbProjectorCommand"];
  WKUserScript *bootstrap = [[WKUserScript alloc]
      initWithSource:[self projectorBootstrapScript]
      injectionTime:WKUserScriptInjectionTimeAtDocumentStart
      forMainFrameOnly:YES];
  [contentController addUserScript:bootstrap];
  configuration.userContentController = contentController;
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

  NSURL *projectorURL = [NSURL URLWithString:@"sdb-projector://localhost/projector.html"];
  [webView loadRequest:[NSURLRequest requestWithURL:projectorURL]];
  [self publishDisplayCount];
}

- (NSString *)projectorBootstrapScript {
  return @"(function(){"
          "let state=null,waiters=[],listeners=[],pending=new Map(),nextId=1;"
          "function visibility(payload){const host=payload&&payload.queries&&payload.queries['/api/v2/host'];"
          "document.documentElement.style.visibility=!host||host.projector_output==='external_display'?'visible':'hidden';}"
          "const bridge={"
          "bootstrap(){if(state)return Promise.resolve(state.envelope);return new Promise(resolve=>waiters.push(resolve));},"
          "query(path){return this.bootstrap().then(()=>state.queries[path]);},"
          "subscribe(listener){listeners.push(listener);return()=>{listeners=listeners.filter(item=>item!==listener);};},"
          "dispatch(envelope){return new Promise((resolve,reject)=>{const id=nextId++;pending.set(id,{resolve,reject});"
          "window.webkit.messageHandlers.sdbProjectorCommand.postMessage({id:id,envelope:envelope});});},"
          "acknowledgeEffect(effectId){return new Promise((resolve,reject)=>{const id=nextId++;pending.set(id,{resolve,reject});"
          "window.webkit.messageHandlers.sdbProjectorCommand.postMessage({id:id,effectId:effectId});});},"
          "receive(payload){state=payload;visibility(payload);const queued=waiters;waiters=[];"
          "queued.forEach(resolve=>resolve(payload.envelope));listeners.forEach(listener=>listener(payload));},"
          "commandResult(id,response){if(response.payload)this.receive(response.payload);const request=pending.get(id);"
          "if(!request)return;pending.delete(id);if(response.ok)request.resolve(response.result);"
          "else request.reject(new Error(response.error||'External projector command failed'));}"
          "};window.__SDB_EXTERNAL_PROJECTOR__=bridge;})();";
}

- (void)webView:(WKWebView *)webView
    startURLSchemeTask:(id<WKURLSchemeTask>)urlSchemeTask {
  NSURLRequest *request = urlSchemeTask.request;
  if (![request.HTTPMethod isEqualToString:@"GET"] ||
      ![request.URL.host isEqualToString:@"localhost"]) {
    NSError *error = [NSError errorWithDomain:@"SmartDartboardProjector"
                                         code:403
                                     userInfo:@{NSLocalizedDescriptionKey : @"Projector request rejected"}];
    [urlSchemeTask didFailWithError:error];
    return;
  }
  NSString *path = urlSchemeTask.request.URL.path ?: @"/projector.html";
  SDBProjectorAsset asset = {0};
  if (!sdb_projector_asset(path.UTF8String, &asset)) {
    NSError *error = [NSError errorWithDomain:@"SmartDartboardProjector"
                                         code:404
                                     userInfo:@{NSLocalizedDescriptionKey : @"Projector asset not found"}];
    [urlSchemeTask didFailWithError:error];
    return;
  }
  NSString *mime = asset.mime == nullptr ? @"application/octet-stream"
                                          : [NSString stringWithUTF8String:asset.mime];
  NSData *data = [NSData dataWithBytes:asset.data length:asset.length];
  sdb_projector_asset_free(asset);
  NSURLResponse *response = [[NSURLResponse alloc]
      initWithURL:urlSchemeTask.request.URL
      MIMEType:mime
      expectedContentLength:(NSInteger)data.length
      textEncodingName:nil];
  [urlSchemeTask didReceiveResponse:response];
  [urlSchemeTask didReceiveData:data];
  [urlSchemeTask didFinish];
}

- (void)webView:(WKWebView *)webView
    stopURLSchemeTask:(id<WKURLSchemeTask>)urlSchemeTask {
  (void)webView;
  (void)urlSchemeTask;
}

- (void)userContentController:(WKUserContentController *)userContentController
      didReceiveScriptMessage:(WKScriptMessage *)message {
  (void)userContentController;
  if (![message.name isEqualToString:@"sdbProjectorCommand"] ||
      !message.frameInfo.isMainFrame ||
      ![message.body isKindOfClass:NSDictionary.class]) {
    return;
  }
  NSDictionary *body = message.body;
  NSNumber *requestID = body[@"id"];
  NSDictionary *envelope = body[@"envelope"];
  NSString *effectID = body[@"effectId"];
  BOOL hasEnvelope = [envelope isKindOfClass:NSDictionary.class] &&
                     [NSJSONSerialization isValidJSONObject:envelope];
  BOOL hasEffect = [effectID isKindOfClass:NSString.class] && effectID.length > 0 &&
                   effectID.length <= 256;
  if (![requestID isKindOfClass:NSNumber.class] || hasEnvelope == hasEffect) {
    return;
  }
  unsigned long long requestNumber = requestID.unsignedLongLongValue;
  if (requestNumber == 0) {
    return;
  }
  char *resultBytes = nullptr;
  if (hasEnvelope) {
    NSData *commandData = [NSJSONSerialization dataWithJSONObject:envelope options:0 error:nil];
    NSString *command = [[NSString alloc] initWithData:commandData
                                              encoding:NSUTF8StringEncoding];
    resultBytes = sdb_projector_command(command.UTF8String);
  } else {
    resultBytes = sdb_projector_effect_ack(effectID.UTF8String);
  }
  if (resultBytes == nullptr) {
    return;
  }
  NSString *resultJSON = [NSString stringWithUTF8String:resultBytes];
  sdb_projector_string_free(resultBytes);
  NSString *script = [NSString stringWithFormat:
      @"window.__SDB_EXTERNAL_PROJECTOR__.commandResult(%llu,%@);",
      requestNumber, resultJSON];
  [message.webView evaluateJavaScript:script completionHandler:nil];
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
  (void)navigation;
  [self applyLatestStateToWebView:webView];
}

- (void)applyLatestStateToWebView:(WKWebView *)webView {
  if (self.latestStateJSON == nil) {
    return;
  }
  NSString *script = [NSString stringWithFormat:
      @"window.__SDB_EXTERNAL_PROJECTOR__.receive(%@);", self.latestStateJSON];
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
