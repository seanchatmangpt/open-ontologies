module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  moduleNameMapper: {
    '^@truex/vkg$': '<rootDir>/packages/truex/vkg/index.ts',
    '^@truex/cli$': '<rootDir>/packages/truex/cli/index.ts',
    '^@truex/examples$': '<rootDir>/packages/truex/examples/index.ts',
    '^@truex/conformance$': '<rootDir>/packages/truex/conformance/index.ts',
    '^@truex/replay$': '<rootDir>/packages/truex/replay/index.ts',
    '^@truex/otel$': '<rootDir>/packages/truex/otel/index.ts',
    '^@truex/capture$': '<rootDir>/packages/truex/capture/index.ts',
    '^@truex/verifier$': '<rootDir>/packages/truex/verifier/index.ts',
    '^@truex/receipt$': '<rootDir>/packages/truex/receipt/index.ts',
    '^@truex/canonical$': '<rootDir>/packages/truex/canonical/index.ts',
    '^@truex/ocel2$': '<rootDir>/packages/truex/ocel2/index.ts',
    '^@truex/(.*)$': '<rootDir>/packages/truex/$1/index.ts'
  }
};
