# ZeroPay Website

The official landing page for ZeroPay - an open-source, self-hosted payment gateway for stablecoins and crypto subscriptions.

## 🚀 Quick Start

### Prerequisites

- Node.js 18+
- npm or yarn package manager

### Installation

1. Clone the repository:
```bash
git clone https://github.com/ZeroPayDev/ZeroPay.git
cd ZeroPay
```

2. Install dependencies:
```bash
npm install
# or
yarn install
```

3. Start the development server:
```bash
npm run dev
# or
yarn dev
```

4. Open [http://localhost:3000](http://localhost:3000) in your browser.

## 🛠 Development

### Project Structure

```
zeropay-website/
├── src/
│   ├── components/          # Reusable React components
│   ├── pages/              # Page components
│   ├── styles/             # CSS and styling files
│   └── utils/              # Utility functions
├── public/                 # Static assets
├── App.jsx                 # Main application component
├── package.json           # Project dependencies and scripts
└── README.md             # This file
```

### Available Scripts

- `npm run dev` - Start development server with hot reload
- `npm run build` - Build production-ready bundle
- `npm run preview` - Preview production build locally
- `npm run lint` - Run ESLint for code quality
- `npm run lint:fix` - Fix auto-fixable linting issues
- `npm run format` - Format code with Prettier

### Development Workflow

1. **Feature Development**
   - Create a new branch: `git checkout -b feature/your-feature-name`
   - Make your changes
   - Test locally: `npm run dev`
   - Lint your code: `npm run lint`
   - Commit changes: `git commit -m "feat: add your feature"`

2. **Code Quality**
   - Follow React best practices
   - Use TypeScript for type safety (if applicable)
   - Ensure responsive design with Tailwind CSS
   - Write clean, semantic HTML
   - Optimize for accessibility (a11y)

3. **Testing**
   - Test on multiple devices and screen sizes
   - Verify all links work correctly
   - Check loading performance
   - Validate HTML and CSS

## 🏗 Build & Deployment

### Production Build

Create an optimized production build:

```bash
npm run build
```

The build artifacts will be stored in the `dist/` directory (Vite) or `build/` directory (Create React App).

### Build Optimization

The production build includes:
- ✅ Minified JavaScript and CSS
- ✅ Tree-shaking for smaller bundle size
- ✅ Asset optimization
- ✅ Source maps for debugging
- ✅ Progressive Web App features (if configured)

### Deployment Options

#### Static Site Hosting

**Vercel (Recommended)**
```bash
npm install -g vercel
vercel --prod
```

**Netlify**
```bash
npm run build
# Drag and drop the dist/ folder to netlify.com
```

**GitHub Pages**
```bash
npm run build
# Push the dist/ folder to gh-pages branch
```

#### Server Deployment

**Traditional Web Server**
```bash
npm run build
# Upload dist/ contents to your web server
```

**Docker**
```dockerfile
FROM nginx:alpine
COPY dist/ /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Environment Configuration

Create environment files for different stages:

**`.env.local` (Development)**
```env
VITE_API_URL=http://localhost:8080
VITE_GITHUB_URL=https://github.com/ZeroPayDev/ZeroPay
```

**`.env.production` (Production)**
```env
VITE_API_URL=https://api.zeropay.dev
VITE_GITHUB_URL=https://github.com/ZeroPayDev/ZeroPay
```

## 🔧 Configuration

### Vite Configuration

If using Vite, customize `vite.config.js`:

```javascript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
        },
      },
    },
  },
  server: {
    port: 3000,
    open: true,
  },
})
```

### Tailwind CSS Configuration

Customize styling in `tailwind.config.js`:

```javascript
module.exports = {
  content: ['./src/**/*.{js,jsx,ts,tsx}', './App.jsx'],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        },
      },
    },
  },
  plugins: [],
}
```

## 📦 Release Process

### Version Management

1. **Update version**:
```bash
npm version patch|minor|major
```

2. **Create release notes**:
   - Document new features
   - List bug fixes
   - Note breaking changes

3. **Tag release**:
```bash
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin v1.0.0
```

### Automated Release (GitHub Actions)

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: npm ci

      - name: Build
        run: npm run build

      - name: Deploy to production
        run: |
          # Add your deployment commands here
          echo "Deploying to production..."
```

### Pre-release Checklist

- [ ] All tests pass
- [ ] Code is linted and formatted
- [ ] Performance is optimized
- [ ] Accessibility is verified
- [ ] Cross-browser testing completed
- [ ] Mobile responsiveness confirmed
- [ ] SEO meta tags updated
- [ ] Analytics tracking verified

## 🤝 Contributing

### Getting Started

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

### Contribution Guidelines

- **Code Style**: Follow the existing code style and use Prettier/ESLint
- **Commits**: Use conventional commit messages (`feat:`, `fix:`, `docs:`, etc.)
- **Testing**: Ensure your changes don't break existing functionality
- **Documentation**: Update README and comments as needed

### Issue Reporting

When reporting issues, please include:
- Browser and version
- Screen size/device
- Steps to reproduce
- Expected vs actual behavior
- Screenshots (if applicable)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Main Project**: [ZeroPay](https://github.com/ZeroPayDev/ZeroPay)
- **Documentation**: [docs.zeropay.dev](https://docs.zeropay.dev)
- **Website**: [zeropay.dev](https://zeropay.dev)
- **Issues**: [GitHub Issues](https://github.com/ZeroPayDev/ZeroPay/issues)

## 📧 Support

For questions and support:
- Create an [issue](https://github.com/ZeroPayDev/ZeroPay/issues)
- Join our [Discord community](https://discord.gg/zeropay)
- Email: hi@zeropay.dev

---

Made with ❤️ by the ZeroPay community
