/**
 * Provides infrastructure for tests that would require mock document.
 */

"use strict";

this.EXPORTED_SYMBOLS = ["MockDocument"];

Cu.importGlobalProperties(["URL"]);
ChromeUtils.import("resource://gre/modules/XPCOMUtils.jsm");

const { NetUtil } = ChromeUtils.import("resource://gre/modules/NetUtil.jsm", {});

const MockDocument = {
  /**
   * Create a document for the given URL containing the given HTML with the ownerDocument of all <form>s having a mocked location.
   */
  createTestDocument(aDocumentURL, aContent = "<form>", aType = "text/html") {
    let parser = Cc["@mozilla.org/xmlextras/domparser;1"].
                 createInstance(Ci.nsIDOMParser);
    parser.init();
    let parsedDoc = parser.parseFromString(aContent, aType);

    // Assign ownerGlobal to documentElement as well for the form-less
    // inputs treating it as rootElement.
    this.mockOwnerGlobalProperty(parsedDoc.documentElement);

    for (let form of parsedDoc.forms) {
      this.mockOwnerDocumentProperty(form, parsedDoc, aDocumentURL);
      this.mockOwnerGlobalProperty(form);
      for (let field of form.elements) {
        this.mockOwnerGlobalProperty(field);
      }
    }
    return parsedDoc;
  },

  mockOwnerDocumentProperty(aElement, aDoc, aURL) {
    // Mock the document.location object so we can unit test without a frame. We use a proxy
    // instead of just assigning to the property since it's not configurable or writable.
    let document = new Proxy(aDoc, {
      get(target, property, receiver) {
        // document.location is normally null when a document is outside of a "browsing context".
        // See https://html.spec.whatwg.org/#the-location-interface
        if (property == "location") {
          return new URL(aURL);
        }
        return target[property];
      },
    });

    // Assign element.ownerDocument to the proxy so document.location works.
    Object.defineProperty(aElement, "ownerDocument", {
      value: document,
    });
  },

  mockOwnerGlobalProperty(aElement) {
    Object.defineProperty(aElement, "ownerGlobal", {
      value: {
        QueryInterface: XPCOMUtils.generateQI([Ci.nsIInterfaceRequestor]),
        getInterface: () => ({
          addManuallyManagedState() {},
          removeManuallyManagedState() {},
        }),
        UIEvent: Event,
        Event,
      },
      configurable: true,
    });
  },

  createTestDocumentFromFile(aDocumentURL, aFile) {
    let fileStream = Cc["@mozilla.org/network/file-input-stream;1"].
                     createInstance(Ci.nsIFileInputStream);
    fileStream.init(aFile, -1, -1, 0);

    let data = NetUtil.readInputStreamToString(fileStream, fileStream.available());

    return this.createTestDocument(aDocumentURL, data);
  },

};

