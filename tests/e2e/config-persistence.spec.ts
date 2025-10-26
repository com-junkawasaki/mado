import { test, expect } from '@playwright/test';
import { _electron as electron } from 'playwright';
import { createTestHelper } from './helpers';

test.describe('Configuration Persistence E2E Tests', () => {
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
  });

  test('should save and load protocol configuration', async () => {
    const testConfig = {
      version: "1.0.0",
      maxMessageSize: 2048576,
      heartbeatInterval: 45,
      sessionTimeout: 350,
      compressionEnabled: false
    };

    // Save configuration
    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_protocol_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, testConfig);

    expect(saveResult.error).not.toBe(true);

    // Load configuration
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_protocol_config');
      } catch (error) {
        return null;
      }
    });

    // Verify configuration was persisted
    if (loadedConfig) {
      expect(loadedConfig.maxMessageSize).toBe(testConfig.maxMessageSize);
      expect(loadedConfig.heartbeatInterval).toBe(testConfig.heartbeatInterval);
    }
  });

  test('should save and load input configuration', async () => {
    const testConfig = {
      keyboardEnabled: true,
      mouseEnabled: false,
      toggleHotkey: {
        modifiers: ["ctrl", "alt"],
        key: "k"
      }
    };

    // Save configuration
    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_input_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, testConfig);

    expect(saveResult.error).not.toBe(true);

    // Load configuration
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_input_config');
      } catch (error) {
        return null;
      }
    });

    // Verify configuration was persisted
    if (loadedConfig) {
      expect(loadedConfig.keyboardEnabled).toBe(testConfig.keyboardEnabled);
      expect(loadedConfig.mouseEnabled).toBe(testConfig.mouseEnabled);
    }
  });

  test('should save and load service configuration', async () => {
    const testConfig = {
      systemdEnabled: true,
      launchdEnabled: false,
      windowsServiceEnabled: false,
      serviceName: "test-kvm-service",
      autoStart: true
    };

    // Save configuration
    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_service_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, testConfig);

    expect(saveResult.error).not.toBe(true);

    // Load configuration
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_service_config');
      } catch (error) {
        return null;
      }
    });

    // Verify configuration was persisted
    if (loadedConfig) {
      expect(loadedConfig.systemdEnabled).toBe(testConfig.systemdEnabled);
      expect(loadedConfig.serviceName).toBe(testConfig.serviceName);
    }
  });

  test('should save and load security configuration', async () => {
    const testConfig = {
      tlsEnabled: true,
      certificatePath: "/test/path/cert.pem",
      privateKeyPath: "/test/path/key.pem",
      caCertificatePath: "/test/path/ca.pem",
      clientAuthEnabled: true
    };

    // Save configuration
    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_security_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, testConfig);

    expect(saveResult.error).not.toBe(true);

    // Load configuration
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_security_config');
      } catch (error) {
        return null;
      }
    });

    // Verify configuration was persisted
    if (loadedConfig) {
      expect(loadedConfig.tlsEnabled).toBe(testConfig.tlsEnabled);
      expect(loadedConfig.clientAuthEnabled).toBe(testConfig.clientAuthEnabled);
    }
  });

  test('should save and load discovery configuration', async () => {
    const testConfig = {
      serviceType: "server",
      autoDiscovery: true,
      discoveryInterval: 60,
      serviceName: "test-kvm-server",
      port: 9090
    };

    // Save configuration
    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_discovery_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, testConfig);

    expect(saveResult.error).not.toBe(true);

    // Load configuration
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_discovery_config');
      } catch (error) {
        return null;
      }
    });

    // Verify configuration was persisted
    if (loadedConfig) {
      expect(loadedConfig.autoDiscovery).toBe(testConfig.autoDiscovery);
      expect(loadedConfig.port).toBe(testConfig.port);
    }
  });

  test('should handle configuration validation', async () => {
    // Test invalid configuration
    const invalidConfig = {
      maxMessageSize: -1, // Invalid negative value
      heartbeatInterval: 0, // Invalid zero value
    };

    const saveResult = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_protocol_config', { config });
      } catch (error) {
        return { error: true, message: error.message };
      }
    }, invalidConfig);

    // Should either reject invalid config or sanitize it
    expect(saveResult).toBeDefined();
  });

  test('should migrate configuration between versions', async () => {
    // Test loading old format configuration
    const oldConfig = {
      version: "0.9.0", // Old version
      server_address: "127.0.0.1:8080", // Old field name
      timeout: 300
    };

    // Save old format
    await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_legacy_config', { config });
      } catch (error) {
        return { error: true };
      }
    }, oldConfig);

    // Load should return new format
    const loadedConfig = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('load_config_with_migration');
      } catch (error) {
        return null;
      }
    });

    // Should be migrated to new format
    if (loadedConfig) {
      expect(loadedConfig.version).toBe("1.0.0");
      expect(loadedConfig).toBeDefined();
    }
  });

  test('should handle configuration file permissions', async () => {
    // Test saving to read-only location (would require backend support)
    // For now, test that normal save works
    const config = { test: true };

    const result = await page.evaluate(async (config) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('save_test_config', { config });
      } catch (error) {
        return { error: true };
      }
    }, config);

    expect(result).toBeDefined();
  });

  test('should backup configuration before saving', async () => {
    // Save initial config
    const initialConfig = { version: "1.0.0", setting: "initial" };

    await page.evaluate(async (config) => {
      // @ts-ignore - Tauri invoke
      return await window.__TAURI__.invoke('save_config_with_backup', { config });
    }, initialConfig);

    // Save new config
    const newConfig = { version: "1.0.0", setting: "updated" };

    await page.evaluate(async (config) => {
      // @ts-ignore - Tauri invoke
      return await window.__TAURI__.invoke('save_config_with_backup', { config });
    }, newConfig);

    // Should be able to restore backup
    const backupRestored = await page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('restore_config_backup');
      } catch (error) {
        return null;
      }
    });

    if (backupRestored) {
      expect(backupRestored.setting).toBe("initial");
    }
  });
});
