/**
 * Environment configuration for Sky-Code
 */

function setupEnv() {
  // Default environment variables
  const defaults = {
    FILANTHROPIC_BASE_URL: 'http://localhost:4000',
    FILANTHROPIC_API_KEY: 'ollama',
    FILANTHROPIC_MODEL: 'llama3.1:8b',
  };

  for (const [key, value] of Object.entries(defaults)) {
    if (!process.env[key]) {
      process.env[key] = value;
    }
  }

  return process.env;
}

function getConfig() {
  return {
    baseUrl: process.env.FILANTHROPIC_BASE_URL || 'http://localhost:4000',
    apiKey: process.env.FILANTHROPIC_API_KEY || 'ollama',
    model: process.env.FILANTHROPIC_MODEL || 'llama3.1:8b',
  };
}

module.exports = { setupEnv, getConfig };
