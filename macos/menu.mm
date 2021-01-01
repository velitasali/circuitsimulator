#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include <string.h>

#include "cs_macos.h"

static cs_macos_path_fn g_trigger = NULL;
static cs_macos_path_fn g_about_to_show = NULL;

@interface CSMenu : NSMenu
@property(copy) NSString* csPath;
@end

@implementation CSMenu
@end

@interface CSMenuDelegate : NSObject <NSMenuDelegate>
@end

@implementation CSMenuDelegate
- (void)menuNeedsUpdate:(NSMenu*)menu {
    if (![menu isKindOfClass:[CSMenu class]])
        return;
    CSMenu* m = (CSMenu*)menu;
    if (g_about_to_show && m.csPath.length)
        g_about_to_show(m.csPath.UTF8String);
}
@end

static CSMenuDelegate* g_menu_delegate = nil;

static NSString* CSStripAmpersand(NSString* s) {
    if (!s)
        return @"";
    return [s stringByReplacingOccurrencesOfString:@"&" withString:@""];
}

static void CSApplyShortcut(NSMenuItem* item, NSString* shortcut) {
    if (shortcut.length == 0)
        return;
    NSEventModifierFlags flags = 0;
    NSString* key = shortcut;
    NSArray* parts = [shortcut componentsSeparatedByString:@"+"];
    if (parts.count > 1) {
        key = parts.lastObject;
        for (NSUInteger i = 0; i + 1 < parts.count; i++) {
            NSString* p = [parts[i] lowercaseString];
            if ([p isEqualToString:@"ctrl"] || [p isEqualToString:@"cmd"] || [p isEqualToString:@"command"])
                flags |= NSEventModifierFlagCommand;
            else if ([p isEqualToString:@"shift"])
                flags |= NSEventModifierFlagShift;
            else if ([p isEqualToString:@"alt"] || [p isEqualToString:@"option"])
                flags |= NSEventModifierFlagOption;
            else if ([p isEqualToString:@"meta"] || [p isEqualToString:@"control"])
                flags |= NSEventModifierFlagControl;
        }
    } else {
        flags = NSEventModifierFlagCommand;
    }
    NSString* equiv = key.lowercaseString;
    if ([equiv isEqualToString:@"tab"])
        equiv = @"\t";
    else if ([equiv isEqualToString:@"escape"] || [equiv isEqualToString:@"esc"])
        equiv = @"\e";
    else if ([equiv isEqualToString:@"space"])
        equiv = @" ";
    else if (equiv.length >= 2 && [equiv hasPrefix:@"f"]) {
        int fnum = [[equiv substringFromIndex:1] intValue];
        if (fnum >= 1 && fnum <= 12) {
            unichar fchar = NSF1FunctionKey + (fnum - 1);
            equiv = [NSString stringWithCharacters:&fchar length:1];
            if (parts.count == 1)
                flags = 0;
        }
    }
    item.keyEquivalent = equiv;
    item.keyEquivalentModifierMask = flags;
}

@interface CSMenuTarget : NSObject
- (void)triggered:(id)sender;
@end

@implementation CSMenuTarget
- (void)triggered:(id)sender {
    NSMenuItem* item = sender;
    NSString* path = item.representedObject;
    if (g_trigger && path)
        g_trigger(path.UTF8String);
}
@end

static CSMenuTarget* g_menu_target = nil;

// Qt gives QAction a TextHeuristicRole by default, so on macOS the C++ app
// got "About …" items, "Settings" (Preferences) and "Quit" moved into the
// application menu automatically. qt-bridge hands us raw JSON instead, so we
// match that behaviour here by item id (ids survive translation, unlike text).
static BOOL CSIsAboutItem(NSString* idStr) {
    return [idStr isEqualToString:@"help.about"] || [idStr isEqualToString:@"help.aboutQt"];
}
static BOOL CSIsPreferencesItem(NSString* idStr) {
    return [idStr isEqualToString:@"file.settings"];
}
static BOOL CSIsQuitItem(NSString* idStr) {
    return [idStr isEqualToString:@"file.quit"];
}

// Pulls the role items out of the tree (recursing into submenus) and returns
// what is left. Only arrays are rebuilt; the item dicts themselves are shared.
static NSArray* CSCollectRoleItems(NSArray* items,
                                   NSMutableArray* about,
                                   NSMutableArray* prefs,
                                   NSMutableArray* quit) {
    NSMutableArray* kept = [NSMutableArray array];
    for (id raw in items) {
        if (![raw isKindOfClass:[NSDictionary class]]) {
            [kept addObject:raw];
            continue;
        }
        NSDictionary* d = raw;
        NSString* idStr = d[@"id"] ?: @"";
        NSArray* sub = [d[@"submenu"] isKindOfClass:[NSArray class]] ? d[@"submenu"] : nil;
        if (sub) {
            NSArray* newSub = CSCollectRoleItems(sub, about, prefs, quit);
            if (newSub.count != sub.count) {
                NSMutableDictionary* m = [d mutableCopy];
                m[@"submenu"] = newSub;
                d = m;
            }
        }
        if (CSIsAboutItem(idStr)) {
            [about addObject:d];
        } else if (CSIsPreferencesItem(idStr)) {
            [prefs addObject:d];
        } else if (CSIsQuitItem(idStr)) {
            [quit addObject:d];
        } else {
            [kept addObject:d];
        }
    }
    return kept;
}

// Removing menu entries can leave doubled or dangling separators; Qt cleans
// those up when a QMenu is mirrored, so do the same here.
static NSArray* CSCollapseSeparators(NSArray* items) {
    NSMutableArray* out = [NSMutableArray array];
    for (id raw in items) {
        BOOL sep = [raw isKindOfClass:[NSDictionary class]] && [((NSDictionary*)raw)[@"separator"] boolValue];
        BOOL prevSep = out.count > 0
            && [out.lastObject isKindOfClass:[NSDictionary class]]
            && [((NSDictionary*)out.lastObject)[@"separator"] boolValue];
        if (sep && (prevSep || out.count == 0))
            continue;
        [out addObject:raw];
    }
    while (out.count
           && [out.lastObject isKindOfClass:[NSDictionary class]]
           && [((NSDictionary*)out.lastObject)[@"separator"] boolValue]) {
        [out removeLastObject];
    }
    return out;
}

static NSMenu* CSBuildMenu(NSArray* items, NSString* path) {
    CSMenu* menu = [[CSMenu alloc] initWithTitle:@""];
    menu.csPath = path ?: @"";
    menu.autoenablesItems = NO;
    if (!g_menu_delegate)
        g_menu_delegate = [[CSMenuDelegate alloc] init];
    menu.delegate = g_menu_delegate;
    if (!g_menu_target)
        g_menu_target = [[CSMenuTarget alloc] init];

    for (id raw in items) {
        if (![raw isKindOfClass:[NSDictionary class]])
            continue;
        NSDictionary* d = raw;
        if ([d[@"separator"] boolValue]) {
            [menu addItem:[NSMenuItem separatorItem]];
            continue;
        }
        if (d[@"visible"] && ![d[@"visible"] boolValue])
            continue;
        NSString* text = CSStripAmpersand(d[@"text"] ?: @"");
        NSString* itemPath = d[@"path"] ?: @"";
        NSArray* sub = d[@"submenu"];
        // Leaves omit "submenu"; an array (including empty) is a real submenu.
        BOOL hasSub = [sub isKindOfClass:[NSArray class]];

        BOOL isQuit = CSIsQuitItem(d[@"id"] ?: @"");
        NSMenuItem* item = [[NSMenuItem alloc] initWithTitle:text
                                                      action:hasSub ? nil : (isQuit ? @selector(terminate:) : @selector(triggered:))
                                               keyEquivalent:@""];
        item.target = hasSub ? nil : (isQuit ? nil : g_menu_target);
        item.representedObject = itemPath;
        item.enabled = d[@"enabled"] ? [d[@"enabled"] boolValue] : YES;
        if ([d[@"checkable"] boolValue])
            item.state = [d[@"checked"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
        CSApplyShortcut(item, d[@"shortcut"] ?: @"");
        if (hasSub) {
            NSMenu* child = CSBuildMenu(sub, itemPath);
            child.title = text;
            item.submenu = child;
        }
        [menu addItem:item];
    }
    return menu;
}

void cs_macos_set_menu_callbacks(cs_macos_path_fn trigger, cs_macos_path_fn about_to_show) {
    g_trigger = trigger;
    g_about_to_show = about_to_show;
}

void cs_macos_set_menu(const char* json) {
    if (!json)
        return;
    NSData* data = [NSData dataWithBytes:json length:strlen(json)];
    id parsed = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if (![parsed isKindOfClass:[NSArray class]])
        return;

    NSMutableArray* about = [NSMutableArray array];
    NSMutableArray* prefs = [NSMutableArray array];
    NSMutableArray* quit = [NSMutableArray array];
    NSMutableArray* menus = [NSMutableArray array];
    for (id raw in (NSArray*)parsed) {
        if (![raw isKindOfClass:[NSDictionary class]])
            continue;
        NSMutableDictionary* d = [(NSDictionary*)raw mutableCopy];
        NSArray* items = d[@"items"];
        if (![items isKindOfClass:[NSArray class]])
            items = @[];
        items = CSCollapseSeparators(CSCollectRoleItems(items, about, prefs, quit));
        d[@"items"] = items;
        [menus addObject:d];
    }

    // The application menu: About items, Preferences, Quit. macOS ignores the
    // title of the first submenu and shows the app name instead. Item paths
    // are unchanged, so triggers still route to AppMenuBar.
    NSMutableArray* appItems = [NSMutableArray array];
    [appItems addObjectsFromArray:about];
    if (appItems.count && prefs.count)
        [appItems addObject:@{@"separator" : @YES}];
    [appItems addObjectsFromArray:prefs];
    if (appItems.count && quit.count)
        [appItems addObject:@{@"separator" : @YES}];
    [appItems addObjectsFromArray:quit];
    if (appItems.count) {
        NSDictionary* appMenu = [@{
            @"title" : @"Circuit Simulator",
            @"path" : @"",
            @"items" : appItems,
        } mutableCopy];
        [menus insertObject:appMenu atIndex:0];
    }

    NSMenu* mainMenu = [[NSMenu alloc] initWithTitle:@""];
    for (id raw in menus) {
        if (![raw isKindOfClass:[NSDictionary class]])
            continue;
        NSDictionary* d = raw;
        NSString* title = CSStripAmpersand(d[@"title"] ?: @"");
        NSString* path = d[@"path"] ?: @"";
        NSArray* items = d[@"items"];
        if (![items isKindOfClass:[NSArray class]])
            items = @[];
        NSMenu* menu = CSBuildMenu(items, path);
        menu.title = title;
        NSMenuItem* top = [[NSMenuItem alloc] initWithTitle:title action:nil keyEquivalent:@""];
        top.submenu = menu;
        [mainMenu addItem:top];
    }
    NSApp.mainMenu = mainMenu;
}
