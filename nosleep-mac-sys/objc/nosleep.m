#import "nosleep.h"

// Reference
// https://developer.apple.com/documentation/iokit/1557134-iopmassertioncreatewithname
// https://developer.apple.com/library/archive/qa/qa1340/_index.html

// Disables the system to enter power save mode
// and inserts the handle as parameter.
// Returns 0 on success
int start(NSString *noSleepType, IOPMAssertionID *handle) {
  @autoreleasepool {
    return IOPMAssertionCreateWithName((__bridge CFStringRef)noSleepType,
                                       kIOPMAssertionLevelOn, assertionName,
                                       handle);
  }
}

// Re-enables power save mode
void stop(IOPMAssertionID handle) {
  @autoreleasepool {
    IOPMAssertionRelease(handle);
  }
}

// Detects if the block handle is still active
bool isStarted(IOPMAssertionID handle) {
  @autoreleasepool {
    NSDictionary *assertions = nil;
    IOPMCopyAssertionsByProcess((CFDictionaryRef *)&assertions);
    NSArray *filteredArray = [[assertions allValues]
        filteredArrayUsingPredicate:
            [NSPredicate
                predicateWithFormat:@"AssertionId CONTAINS %d", handle]];
    return [filteredArray count] > 0;
  }
}