#import <AppKit/AppKit.h>

#include "cs_macos.h"

static NSTouchBarItemIdentifier const CSPowerItem = @"com.circuitsimulator.touchbar.power";
static NSTouchBarItemIdentifier const CSPauseItem = @"com.circuitsimulator.touchbar.pause";

static cs_macos_fn g_power_fn = NULL;
static cs_macos_fn g_pause_fn = NULL;
static cs_macos_state_fn g_power_state = NULL;
static cs_macos_state_fn g_pause_state = NULL;

static NSImage* CSSymbol(NSString* name, NSString* description) {
    if (@available(macOS 11.0, *))
        return [NSImage imageWithSystemSymbolName:name accessibilityDescription:description];
    return nil;
}

@interface CSTouchBarDelegate : NSObject <NSTouchBarDelegate>
@property(strong) NSHashTable<NSButton*>* powerButtons;
@property(strong) NSHashTable<NSButton*>* pauseButtons;
- (NSTouchBar*)buildTouchBar;
- (void)refresh;
- (void)updatePowerButton:(NSButton*)button;
- (void)updatePauseButton:(NSButton*)button;
@end

@implementation CSTouchBarDelegate

- (instancetype)init {
    self = [super init];
    if (self) {
        _powerButtons = [NSHashTable weakObjectsHashTable];
        _pauseButtons = [NSHashTable weakObjectsHashTable];
    }
    return self;
}

- (NSTouchBar*)buildTouchBar {
    NSTouchBar* bar = [[NSTouchBar alloc] init];
    bar.delegate = self;
    bar.customizationIdentifier = @"com.circuitsimulator.touchbar.main";
    bar.defaultItemIdentifiers = @[ CSPowerItem, CSPauseItem ];
    bar.customizationAllowedItemIdentifiers = bar.defaultItemIdentifiers;
    return bar;
}

- (NSTouchBarItem*)touchBar:(NSTouchBar*)touchBar makeItemForIdentifier:(NSTouchBarItemIdentifier)identifier {
    (void)touchBar;
    if ([identifier isEqualToString:CSPowerItem]) {
        NSCustomTouchBarItem* item = [[NSCustomTouchBarItem alloc] initWithIdentifier:identifier];
        item.customizationLabel = @"Start/Stop Simulation";
        NSButton* button = [NSButton buttonWithTitle:@"" target:self action:@selector(powerClicked:)];
        [self.powerButtons addObject:button];
        [self updatePowerButton:button];
        item.view = button;
        return item;
    }
    if ([identifier isEqualToString:CSPauseItem]) {
        NSCustomTouchBarItem* item = [[NSCustomTouchBarItem alloc] initWithIdentifier:identifier];
        item.customizationLabel = @"Pause/Resume Simulation";
        NSButton* button = [NSButton buttonWithTitle:@"" target:self action:@selector(pauseClicked:)];
        [self.pauseButtons addObject:button];
        [self updatePauseButton:button];
        item.view = button;
        return item;
    }
    return nil;
}

- (void)updatePowerButton:(NSButton*)button {
    if (!button)
        return;
    int power = g_power_state ? g_power_state() : 0;
    NSString* pLabel = power ? @"Stop" : @"Run";
    NSString* pSymbol = power ? @"stop.fill" : @"play.fill";
    NSImage* pImage = CSSymbol(pSymbol, pLabel);
    button.image = pImage;
    button.imagePosition = pImage ? NSImageOnly : NSNoImage;
    button.title = pImage ? @"" : pLabel;
    button.enabled = YES;
}

- (void)updatePauseButton:(NSButton*)button {
    if (!button)
        return;
    int pause = g_pause_state ? g_pause_state() : -1;
    BOOL isResume = pause == 1;
    NSString* sLabel = isResume ? @"Resume" : @"Pause";
    NSImage* sImage = CSSymbol(isResume ? @"play.fill" : @"pause.fill", sLabel);
    button.image = sImage;
    button.imagePosition = sImage ? NSImageOnly : NSNoImage;
    button.title = sImage ? @"" : sLabel;
    button.enabled = pause >= 0;
}

- (void)refresh {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            [self refresh];
        });
        return;
    }
    for (NSButton* b in self.powerButtons) {
        [self updatePowerButton:b];
    }
    for (NSButton* b in self.pauseButtons) {
        [self updatePauseButton:b];
    }
}

- (void)powerClicked:(id)sender {
    (void)sender;
    if (g_power_fn)
        g_power_fn();
}

- (void)pauseClicked:(id)sender {
    (void)sender;
    if (g_pause_fn)
        g_pause_fn();
}

@end

static CSTouchBarDelegate* g_delegate = nil;

static void CSAttachToWindow(NSWindow* window) {
    if (!window || !g_delegate)
        return;
    if (@available(macOS 10.12.2, *)) {
        if (!window.touchBar) {
            window.touchBar = [g_delegate buildTouchBar];
        }
    }
}

void cs_macos_install_touchbar(cs_macos_fn power, cs_macos_fn pause, cs_macos_state_fn power_state,
                               cs_macos_state_fn pause_state) {
    g_power_fn = power;
    g_pause_fn = pause;
    g_power_state = power_state;
    g_pause_state = pause_state;

    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        g_delegate = [[CSTouchBarDelegate alloc] init];

        if (@available(macOS 10.12.2, *)) {
            [NSApp setTouchBar:[g_delegate buildTouchBar]];
        }

        [[NSNotificationCenter defaultCenter]
            addObserverForName:NSWindowDidBecomeKeyNotification
                        object:nil
                         queue:[NSOperationQueue mainQueue]
                    usingBlock:^(NSNotification* note) {
                        if ([note.object isKindOfClass:[NSWindow class]]) {
                            CSAttachToWindow((NSWindow*)note.object);
                            [g_delegate refresh];
                        }
                    }];

        [[NSNotificationCenter defaultCenter]
            addObserverForName:NSWindowDidBecomeMainNotification
                        object:nil
                         queue:[NSOperationQueue mainQueue]
                    usingBlock:^(NSNotification* note) {
                        if ([note.object isKindOfClass:[NSWindow class]]) {
                            CSAttachToWindow((NSWindow*)note.object);
                            [g_delegate refresh];
                        }
                    }];
    });

    if (NSApp.mainWindow) {
        CSAttachToWindow(NSApp.mainWindow);
    }
    if (NSApp.keyWindow) {
        CSAttachToWindow(NSApp.keyWindow);
    }
    for (NSWindow* w in NSApp.windows) {
        CSAttachToWindow(w);
    }
    [g_delegate refresh];
}

void cs_macos_refresh_touchbar(void) {
    [g_delegate refresh];
}
