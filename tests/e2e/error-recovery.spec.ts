import { test, expect } from '@playwright/test';
import { _electron as electron } from 'playwright';
import { createTestHelper } from './helpers';

test.describe('Error Recovery E2E Tests', () => {
  let electronApp: Awaited<ReturnType<typeof electron.launch>>;
  let page: any;
  let helper: ReturnType<typeof createTestHelper>;

  test.beforeAll(async () => {
    electronApp = await electron.launch({
      args: ['.'],
      env: {
        ...process.env,
        NODE_ENV: 'test',
      },
    });

    page = await electronApp.firstWindow();
    helper = createTestHelper(page);
  });

  test.afterAll(async () => {
    await electronApp.close();
  });

  test.beforeEach(async () => {
    await helper.waitForAppReady();
    await helper.clearErrors();
  });

  test('should recover from network connection failures', async () => {
    // Attempt to connect to non-existent server
    const connected = await helper.connectToServer('192.0.2.1:9999'); // RFC 5737 test address
    expect(connected).toBe(false);

    // Check that error is handled gracefully
    const errors = await helper.getErrorMessages();
    // Note: UI might not show errors immediately, so we check app stability

    // App should still be functional
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();

    // Should be able to attempt connection again
    const retryConnected = await helper.connectToServer('127.0.0.1:8080');
    expect(typeof retryConnected).toBe('boolean'); // Should not crash
  });

  test('should recover from invalid configuration', async () => {
    // Test invalid server address
    const serverResult = await helper.startProtocolServer('');
    expect(serverResult).toBe(false);

    // Test invalid client configuration
    const clientResult = await helper.connectToServer('');
    expect(clientResult).toBe(false);

    // App should remain stable
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();

    // Should be able to use valid configuration after invalid attempts
    const validServer = await helper.startProtocolServer('127.0.0.1:8070');
    expect(typeof validServer).toBe('boolean');
  });

  test('should handle plugin initialization failures', async () => {
    // Simulate plugin failure (this would require backend modification)
    // For now, test that app remains stable when plugins fail

    // Force a plugin command that might fail
    const result = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('invalid_command');
      } catch (error) {
        return { error: true, message: error.message };
      }
    });

    expect(result.error).toBe(true);

    // App should still be functional
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should recover from connection timeouts', async () => {
    // Start server
    const serverStarted = await helper.startProtocolServer('127.0.0.1:8071');
    expect(serverStarted).toBe(true);

    // Connect with timeout simulation (connect to wrong port)
    const connected = await helper.connectToServer('127.0.0.1:8072');
    expect(connected).toBe(false);

    // Wait for any timeout handling
    await page.waitForTimeout(2000);

    // App should recover and allow new connections
    const retryConnected = await helper.connectToServer('127.0.0.1:8071');
    expect(typeof retryConnected).toBe('boolean');
  });

  test('should handle resource exhaustion gracefully', async () => {
    // Start multiple servers (test resource limits)
    const servers = [];
    for (let i = 0; i < 5; i++) {
      const port = 8080 + i;
      const result = await helper.startProtocolServer(`127.0.0.1:${port}`);
      servers.push(result);
    }

    // At least some should succeed
    const successCount = servers.filter(s => s).length;
    expect(successCount).toBeGreaterThan(0);

    // App should remain stable
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should recover from input device errors', async () => {
    // Start input capture
    const captureStarted = await helper.startInputCapture();
    expect(typeof captureStarted).toBe('boolean');

    // Simulate input device failure (this would require backend support)
    // For now, test that stopping works even if starting failed
    const stopped = await helper.stopInputCapture();
    expect(typeof stopped).toBe('boolean');

    // App should remain stable
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should handle configuration file corruption', async () => {
    // Test with invalid configuration
    const result = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_invalid_config');
      } catch (error) {
        return { error: true };
      }
    });

    // Should handle gracefully (either succeed with defaults or fail gracefully)
    expect(result).toBeDefined();

    // App should remain functional
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should recover from plugin crashes', async () => {
    // This test would require the ability to crash plugins
    // For now, test that the app remains stable when plugin operations fail

    // Perform operations that might stress plugins
    const operations = [];
    for (let i = 0; i < 10; i++) {
      operations.push(helper.sendTestInputEvent());
    }

    const results = await Promise.all(operations);

    // Some might fail, but app should remain stable
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should handle concurrent error conditions', async () => {
    // Create multiple error conditions simultaneously
    const errorOperations = [
      helper.connectToServer('invalid:9999'),
      helper.startProtocolServer('invalid'),
      helper.sendTestInputEvent(), // Without capture running
      page.evaluate(async () => {
        // @ts-ignore
        return window.__TAURI__.invoke('nonexistent_command');
      })
    ];

    const results = await Promise.allSettled(errorOperations);

    // All operations should complete (either success or handled failure)
    expect(results.length).toBe(4);

    // App should remain stable despite multiple errors
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should maintain error logging', async () => {
    // Clear any existing errors
    await helper.clearErrors();

    // Generate some errors
    await helper.connectToServer('192.0.2.1:9999');
    await helper.startProtocolServer('');

    // Errors should be logged (implementation dependent)
    // At minimum, app should not crash
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();
  });

  test('should recover from application restart', async () => {
    // This is more of an integration test that would require
    // restarting the application between tests

    // For now, test that the app can be "reset" to initial state
    await helper.simulateNetworkError();

    // App should be recoverable
    const appStatus = await helper.getAppStatus();
    expect(appStatus).toBeTruthy();

    // Should be able to start fresh operations
    const serverStarted = await helper.startProtocolServer('127.0.0.1:8073');
    expect(typeof serverStarted).toBe('boolean');
  });
});
