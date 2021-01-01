#import <AppKit/AppKit.h>

#include "cs_macos.h"

void cs_macos_apply_app_icon(const char* path) {
    NSBundle* mainBundle = [NSBundle mainBundle];
    NSString* bundlePath = [mainBundle bundlePath];
    BOOL isAppBundle = [bundlePath hasSuffix:@".app"];
    BOOL hasAssetsCar = [mainBundle pathForResource:@"Assets" ofType:@"car"] != nil;

    // In a packaged .app bundle with Assets.car, let macOS Dock and WindowServer
    // dynamically render the Liquid Glass icon stack from Info.plist / Assets.car.
    // Setting NSApp.applicationIconImage would replace the dynamic multi-appearance
    // Liquid Glass icon with a static snapshot, breaking Clear/Dark/Tinted modes.
    if (isAppBundle && hasAssetsCar) {
        NSApp.applicationIconImage = nil;
        return;
    }

    // Otherwise (unbundled binary development), apply the standalone fallback image.
    NSImage* img = nil;
    if (path && path[0]) {
        img = [[NSImage alloc] initWithContentsOfFile:[NSString stringWithUTF8String:path]];
    }
    if (!img) {
        img = [mainBundle imageForResource:@"circuitsimulator"];
    }
    if (!img) {
        img = [NSImage imageNamed:@"circuitsimulator"];
    }
    if (!img) {
        img = [NSImage imageNamed:NSImageNameApplicationIcon];
    }
    if (img) {
        NSApp.applicationIconImage = img;
    }
}
